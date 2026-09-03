// (C) 2025-2026 MWBM Partners Ltd
//
// Structured logging with PII redaction and dated log files.
//
// Uses the `tracing` crate for structured, async-safe logging.
// Console output is human-readable with colours; file output is
// JSON-structured and (optionally) scrubbed of personally-identifiable
// information before it ever touches the disk.
//
// WHY THE FILE WRITER IS HAND-ROLLED: the workspace deliberately does not
// depend on `tracing-appender`, so there is no rolling-file appender to lean
// on. File output is therefore a plain `std::fs::File` opened in append mode
// behind a `Mutex`, chosen ONCE at process start. That gives "append to a
// dated file, picked at startup" — a long-running process that crosses
// midnight keeps writing to the file it opened, it does not roll over. Daily
// *rotation* would need a writer that re-checks the date on every event; the
// dated filename (`meedya-<date>.log`) plus per-run selection is what the
// CLI's short-lived processes actually need.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use sha2::{Digest, Sha256};
use tracing::Level;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::error::{MmError, MmResult};

/// Configuration for the logging system
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Minimum log level for the *file* sink (trace, debug, info, warn, error)
    pub level: String,
    /// Minimum log level for the *console* sink.
    ///
    /// Kept separate from `level` because the two sinks answer to different
    /// masters: the console level is driven by the caller's `-v` flags (a
    /// user-facing CLI must stay quiet by default), whereas the file level
    /// comes from `settings.json5` and is what a bug report will contain.
    /// Sharing one filter would mean a user could not have a verbose log file
    /// without also drowning their terminal.
    pub console_level: String,
    /// Enable console (stdout) logging
    pub console: bool,
    /// Enable file logging
    pub file: bool,
    /// Directory for log files (used when `file_path` is `None`)
    pub log_dir: PathBuf,
    /// Explicit log file path, overriding the dated name inside `log_dir`.
    ///
    /// This is what `settings.json5`'s `logging.file` maps to: the user named
    /// a file, so we write to exactly that file rather than inventing a dated
    /// sibling they would then have to hunt for.
    pub file_path: Option<PathBuf>,
    /// Enable PII redaction in log output
    pub redact_pii: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            console_level: "info".to_string(),
            console: true,
            file: false,
            log_dir: default_log_dir(),
            file_path: None,
            redact_pii: true,
        }
    }
}

/// The platform default directory for MeedyaManager log files.
///
/// Kept as a free function so callers that have no `LogConfig` in hand (the
/// `report-bug` command, for one) can still name the place logs live.
pub fn default_log_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("MeedyaManager")
        .join("logs")
}

impl LogConfig {
    /// Build a `LogConfig` from the `[logging]` section of `settings.json5`.
    ///
    /// This is the bridge that was missing: `AppConfig::logging` was read,
    /// deserialised, round-tripped by the GTK settings UI — and then never
    /// turned into an actual subscriber by anything (issue #50). Mapping is:
    ///
    /// * `logging.file = None`   → file output off.
    /// * `logging.file = Some(p)` → file output on, writing to exactly `p`,
    ///   with `log_dir` set to `p`'s parent so sibling tooling can find it.
    pub fn from_settings(settings: &crate::config::LoggingConfig) -> Self {
        // A configured path both enables file output and names the target —
        // there is no separate on/off flag in settings.json5, so `Some(path)`
        // IS the enable switch. That is what report-bug must now say.
        let file_path = settings.file.clone();

        // Anchor `log_dir` at the configured file's parent when we have one so
        // that directory-scanning consumers look in the right place. An empty
        // parent (a bare "meedya.log") means the current directory.
        let log_dir = file_path.as_ref().map_or_else(default_log_dir, |p| {
            match p.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
                // A relative bare filename has an empty parent; "." is the
                // honest interpretation of where it will land.
                _ => PathBuf::from("."),
            }
        });

        Self {
            level: settings.level.clone(),
            // Default the console to the same level; CLI verbosity flags
            // overwrite this field before `init_logging` is called.
            console_level: settings.level.clone(),
            console: settings.console,
            file: file_path.is_some(),
            log_dir,
            file_path,
            redact_pii: settings.redact_pii,
        }
    }

    /// The file this configuration would log to, or `None` when file output
    /// is disabled.
    ///
    /// Resolution order: an explicit `file_path` wins; otherwise a dated file
    /// (`meedya-YYYY-MM-DD.log`) inside `log_dir`. The date is *today's* —
    /// this is deliberately evaluated per call so a caller asking at 00:01 on
    /// the following day gets the name a fresh process would open.
    pub fn resolved_log_path(&self) -> Option<PathBuf> {
        if !self.file {
            return None;
        }
        Some(self.file_path.clone().unwrap_or_else(|| {
            let date = chrono::Utc::now().format("%Y-%m-%d");
            self.log_dir.join(format!("meedya-{date}.log"))
        }))
    }
}

// ---------------------------------------------------------------------------
// Redacting writer
// ---------------------------------------------------------------------------

/// A `MakeWriter` that appends to an open log file, scrubbing PII on the way.
///
/// WHY REDACT AT THE WRITER RATHER THAN AT EACH CALL SITE: `tracing` events
/// are recorded all over the workspace with raw `Path` values in their fields.
/// Auditing several hundred call sites is neither achievable nor durable — a
/// new `tracing::info!(path = %p)` added next month would silently reintroduce
/// the leak. Filtering the formatted bytes on their way to the file catches
/// every field, every message and every span name in one place.
///
/// WHAT IS NOT COVERED (be honest about the limits):
/// * Console output is *not* redacted. The console belongs to the person
///   already logged in as that user; hiding their own home directory from
///   them would only make diagnostics harder. Only the file — the artefact
///   people attach to bug reports — is scrubbed.
/// * Redaction is textual. It rewrites the current user's home directory and
///   username; it cannot know that some other string is personal data.
/// * Other users' home directories on a shared machine are not rewritten.
#[derive(Clone)]
pub struct RedactingFileWriter {
    /// The append-mode log file, shared between every cloned writer handle.
    file: Arc<Mutex<std::fs::File>>,
    /// Whether to scrub each formatted record before writing it.
    redact: bool,
}

impl RedactingFileWriter {
    /// Wrap an already-open file. The file is expected to be in append mode.
    pub fn new(file: std::fs::File, redact: bool) -> Self {
        Self {
            file: Arc::new(Mutex::new(file)),
            redact,
        }
    }
}

impl std::fmt::Debug for RedactingFileWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The file handle itself is not interesting and the mutex may be
        // locked by another thread; report only the redaction setting.
        f.debug_struct("RedactingFileWriter")
            .field("redact", &self.redact)
            .finish_non_exhaustive()
    }
}

impl std::io::Write for RedactingFileWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // `tracing_subscriber`'s fmt layer formats a whole event into one
        // buffer and hands it over in a single `write_all`, so `buf` is a
        // complete record. That matters: redaction is a whole-string search
        // and would miss a home directory split across two chunks.
        let mut guard = self
            .file
            .lock()
            .map_err(|_| std::io::Error::other("log file mutex poisoned"))?;

        if self.redact {
            // `from_utf8_lossy` rather than a hard error: a malformed byte in
            // a log record must never take down the process it is logging.
            let text = String::from_utf8_lossy(buf);
            guard.write_all(redact_log_record(&text).as_bytes())?;
        } else {
            guard.write_all(buf)?;
        }

        // Report the caller's length, not the redacted length — redaction can
        // shorten the record, and a short count would make `write_all` loop
        // and duplicate the tail.
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file
            .lock()
            .map_err(|_| std::io::Error::other("log file mutex poisoned"))?
            .flush()
    }
}

impl<'a> MakeWriter<'a> for RedactingFileWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        // Cloning shares the same `Arc<Mutex<File>>`, so concurrent events
        // serialise on the mutex and never interleave mid-record.
        self.clone()
    }
}

/// Best-effort discovery of the current user's account name.
///
/// The workspace has no `whoami` dependency and may not add one, so this
/// falls back through the conventional environment variables and finally to
/// the last component of the home directory.
fn current_username() -> Option<String> {
    for key in ["USER", "USERNAME", "LOGNAME"] {
        if let Ok(value) = std::env::var(key)
            && !value.trim().is_empty()
        {
            return Some(value);
        }
    }
    dirs::home_dir()?
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

/// Scrub a formatted log record of the current user's home directory and
/// username.
///
/// Applied to file output only — see `RedactingFileWriter` for the rationale
/// and the list of things this deliberately does not cover.
pub fn redact_log_record(record: &str) -> String {
    let mut out = record.to_string();

    if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy().into_owned();
        // Guard against a degenerate home ("/" or ""), which would rewrite
        // every path separator in the record into a tilde.
        if home_str.len() > 1 {
            // JSON output escapes backslashes, so a Windows home directory
            // appears as `C:\\Users\\alice` inside the record. Replace the
            // escaped form first (it is the longer match) then the raw form,
            // which is what the console-style formatter and span names carry.
            let escaped = home_str.replace('\\', "\\\\");
            if escaped != home_str {
                out = out.replace(&escaped, "~");
            }
            out = out.replace(&home_str, "~");
        }
    }

    if let Some(username) = current_username() {
        // Only substitute names of three characters or more. Shorter names
        // ("j", "ab") occur inside ordinary words and log targets, and
        // rewriting those would corrupt the record for no privacy gain.
        if username.chars().count() >= 3 {
            out = out.replace(&username, &redact_username(&username));
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Subscriber construction and installation
// ---------------------------------------------------------------------------

/// Records the log file chosen by the first successful `init_logging` call.
///
/// A `tracing` global subscriber can be installed exactly once per process,
/// so `init_logging` is made idempotent rather than fallible-on-second-call:
/// a second invocation returns the path the first one settled on.
static ACTIVE_LOG_FILE: OnceLock<Option<PathBuf>> = OnceLock::new();

/// The log file the running process is writing to, if `init_logging` has run
/// and file output was enabled.
pub fn active_log_file() -> Option<PathBuf> {
    ACTIVE_LOG_FILE.get().cloned().flatten()
}

/// Open (creating as needed) the log file this configuration names.
///
/// Returns the resolved path alongside the append-mode handle. Split out from
/// subscriber construction so tests can assert the file-creation half without
/// touching any global subscriber state.
pub fn open_log_file(config: &LogConfig) -> MmResult<(PathBuf, std::fs::File)> {
    let path = config
        .resolved_log_path()
        .ok_or_else(|| MmError::Logging("file logging is not enabled".to_string()))?;

    // Create the containing directory rather than failing on a fresh install
    // where ~/Library/Application Support/MeedyaManager/logs does not exist.
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| MmError::Logging(format!("cannot create log directory: {e}")))?;
    }

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| MmError::Logging(format!("cannot open log file: {e}")))?;

    Ok((path, file))
}

/// Build the subscriber described by `config` without installing it.
///
/// Returned alongside the resolved log file path (if any). Kept separate from
/// `init_logging` so tests can drive a real subscriber through
/// `tracing::subscriber::with_default` — a thread-local install — instead of
/// claiming the process-global slot and fighting every other test.
pub fn build_subscriber(
    config: &LogConfig,
) -> MmResult<(
    impl tracing::Subscriber + Send + Sync + 'static,
    Option<PathBuf>,
)> {
    // Per-sink filters (rather than one filter over the whole registry) are
    // what let the file keep `settings.json5`'s level while the console obeys
    // the CLI's -v count.
    let console_layer = if config.console {
        Some(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false)
                .compact()
                .with_filter(env_filter(&config.console_level)),
        )
    } else {
        None
    };

    let (file_layer, log_path) = if config.file {
        let (path, file) = open_log_file(config)?;
        let layer = tracing_subscriber::fmt::layer()
            .with_writer(RedactingFileWriter::new(file, config.redact_pii))
            // JSON so a bug report is machine-readable; ANSI would only put
            // escape codes in a file nobody views through a terminal.
            .json()
            .with_ansi(false)
            .with_target(true)
            .with_span_events(FmtSpan::CLOSE)
            .with_filter(env_filter(&config.level));
        (Some(layer), Some(path))
    } else {
        (None, None)
    };

    let subscriber = tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer);

    Ok((subscriber, log_path))
}

/// Parse a level string into an `EnvFilter`, falling back to `info`.
///
/// The fallback is silent by design: this runs *while* the subscriber is
/// being built, so a `tracing::warn!` here would go nowhere.
fn env_filter(level: &str) -> EnvFilter {
    EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"))
}

/// Initialise the global tracing subscriber with console and/or file output.
///
/// Call once at application startup, *after* configuration has been loaded —
/// the file sink's path and level both come from `settings.json5`. Returns
/// the log file being written to, or `None` when file output is disabled.
///
/// Idempotent: a second call is a no-op that reports the first call's result,
/// so a test binary or embedder that reaches this twice does not fail.
pub fn init_logging(config: &LogConfig) -> MmResult<Option<PathBuf>> {
    // Fast path: already initialised, hand back what the first call chose.
    if let Some(existing) = ACTIVE_LOG_FILE.get() {
        return Ok(existing.clone());
    }

    let (subscriber, log_path) = build_subscriber(config)?;

    subscriber
        .try_init()
        .map_err(|e| MmError::Logging(format!("cannot initialise logging: {e}")))?;

    // Only record the path once the subscriber is actually installed, so a
    // failed install does not poison later attempts.
    let _ = ACTIVE_LOG_FILE.set(log_path.clone());

    tracing::info!(
        console_level = %config.console_level,
        file_level = %config.level,
        log_file = %log_path.as_ref().map_or_else(|| "(none)".to_string(), |p| p.display().to_string()),
        "Logging initialised"
    );

    Ok(log_path)
}

/// Redact a file path for logging purposes.
///
/// Replaces the home directory portion with `~` and optionally hashes
/// the filename to prevent PII leakage in logs.
pub fn redact_path(path: &Path, hash_filename: bool) -> String {
    let path_str = path.to_string_lossy();

    // Replace home directory with ~
    let redacted = if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy();
        if path_str.starts_with(home_str.as_ref()) {
            path_str.replacen(home_str.as_ref(), "~", 1)
        } else {
            path_str.to_string()
        }
    } else {
        path_str.to_string()
    };

    if hash_filename {
        // Hash just the filename component
        let p = Path::new(&redacted);
        if let (Some(parent), Some(name)) = (p.parent(), p.file_name()) {
            let hash = hash_string(&name.to_string_lossy());
            format!("{}/{}", parent.display(), hash)
        } else {
            redacted
        }
    } else {
        redacted
    }
}

/// Redact a username or personal identifier.
///
/// Replaces all but the first character with asterisks.
///
/// Counts *characters*, not bytes: this is now called with real account names
/// pulled from the environment, and a name whose first character is multi-byte
/// (an accented letter, say) would have panicked on a byte-index slice.
pub fn redact_username(username: &str) -> String {
    let char_count = username.chars().count();
    if char_count == 0 {
        return String::new();
    }
    if char_count <= 2 {
        return "*".repeat(char_count);
    }
    let first: String = username.chars().take(1).collect();
    let rest = "*".repeat(char_count - 1);
    format!("{first}{rest}")
}

/// Generate a short hash of a string for redaction purposes.
///
/// Returns the first 8 hex characters of the SHA-256 hash.
pub fn hash_string(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    format!("{result:x}")[..8].to_string()
}

/// Parse a log level string into a tracing Level.
pub fn parse_level(level: &str) -> Level {
    match level.to_ascii_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" | "warning" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Needed to call `write_all`/`flush` on `RedactingFileWriter` directly;
    // the non-test code reaches those through the trait impl itself.
    use std::io::Write as _;

    #[test]
    fn redact_path_replaces_home() {
        if let Some(home) = dirs::home_dir() {
            let path = home.join("Music").join("song.mp3");
            let redacted = redact_path(&path, false);
            assert!(redacted.starts_with('~'));
            assert!(redacted.contains("Music"));
            assert!(redacted.contains("song.mp3"));
        }
    }

    #[test]
    fn redact_path_with_hash() {
        if let Some(home) = dirs::home_dir() {
            let path = home.join("Music").join("secret_song.mp3");
            let redacted = redact_path(&path, true);
            assert!(redacted.starts_with('~'));
            assert!(!redacted.contains("secret_song"));
        }
    }

    #[test]
    fn redact_path_no_home() {
        let path = Path::new("/opt/media/song.mp3");
        let redacted = redact_path(path, false);
        assert_eq!(redacted, "/opt/media/song.mp3");
    }

    #[test]
    fn redact_username_basic() {
        assert_eq!(redact_username("alice"), "a****");
        assert_eq!(redact_username("bob"), "b**");
    }

    #[test]
    fn redact_username_short() {
        assert_eq!(redact_username("a"), "*");
        assert_eq!(redact_username("ab"), "**");
    }

    #[test]
    fn redact_username_empty() {
        assert_eq!(redact_username(""), "");
    }

    /// A non-ASCII account name must not panic on a byte-index slice — this
    /// function is now reached with whatever the OS calls the current user.
    #[test]
    fn redact_username_multibyte_first_char() {
        assert_eq!(redact_username("émile"), "é****");
        assert_eq!(redact_username("日本語"), "日**");
    }

    #[test]
    fn hash_string_deterministic() {
        let hash1 = hash_string("test");
        let hash2 = hash_string("test");
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 8);
    }

    #[test]
    fn hash_string_different_inputs() {
        let hash1 = hash_string("hello");
        let hash2 = hash_string("world");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn parse_level_valid() {
        assert_eq!(parse_level("trace"), Level::TRACE);
        assert_eq!(parse_level("debug"), Level::DEBUG);
        assert_eq!(parse_level("info"), Level::INFO);
        assert_eq!(parse_level("warn"), Level::WARN);
        assert_eq!(parse_level("warning"), Level::WARN);
        assert_eq!(parse_level("error"), Level::ERROR);
    }

    #[test]
    fn parse_level_case_insensitive() {
        assert_eq!(parse_level("INFO"), Level::INFO);
        assert_eq!(parse_level("Debug"), Level::DEBUG);
        assert_eq!(parse_level("ERROR"), Level::ERROR);
    }

    #[test]
    fn parse_level_unknown_defaults_to_info() {
        assert_eq!(parse_level("invalid"), Level::INFO);
        assert_eq!(parse_level(""), Level::INFO);
    }

    #[test]
    fn default_log_config() {
        let config = LogConfig::default();
        assert_eq!(config.level, "info");
        assert!(config.console);
        assert!(!config.file);
        assert!(config.redact_pii);
    }

    #[test]
    fn log_dir_contains_meedyamanager() {
        let config = LogConfig::default();
        let path_str = config.log_dir.to_string_lossy();
        assert!(path_str.contains("MeedyaManager"));
    }

    // ── settings.json5 → LogConfig bridge (issue #50) ───────────────────

    /// The default `[logging]` section has no file, so file output is off and
    /// there is no resolved path to report.
    #[test]
    fn from_settings_without_file_disables_file_output() {
        let settings = crate::config::LoggingConfig::default();
        let config = LogConfig::from_settings(&settings);
        assert!(!config.file);
        assert_eq!(config.resolved_log_path(), None);
        assert_eq!(config.level, "info");
        assert!(config.console);
    }

    /// `logging.file = "<path>"` is the enable switch, and the resolved path
    /// is exactly what the user wrote — not a dated sibling of it.
    #[test]
    fn from_settings_with_file_enables_exact_path() {
        let settings = crate::config::LoggingConfig {
            level: "debug".to_string(),
            file: Some(PathBuf::from("/var/log/meedya/app.log")),
            redact_pii: false,
            ..crate::config::LoggingConfig::default()
        };

        let config = LogConfig::from_settings(&settings);
        assert!(config.file);
        assert_eq!(config.level, "debug");
        assert_eq!(config.console_level, "debug");
        assert!(!config.redact_pii);
        assert_eq!(config.log_dir, PathBuf::from("/var/log/meedya"));
        assert_eq!(
            config.resolved_log_path(),
            Some(PathBuf::from("/var/log/meedya/app.log"))
        );
    }

    /// Without an explicit path, file output lands on today's dated file
    /// inside `log_dir` — the `meedya-<date>.log` pattern report-bug scans.
    #[test]
    fn resolved_path_falls_back_to_dated_filename() {
        let dir = tempfile::tempdir().unwrap();
        let config = LogConfig {
            file: true,
            log_dir: dir.path().to_path_buf(),
            file_path: None,
            ..LogConfig::default()
        };
        let path = config.resolved_log_path().unwrap();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("meedya-"), "unexpected name: {name}");
        assert!(name.ends_with(".log"), "unexpected name: {name}");
        assert_eq!(path.parent().unwrap(), dir.path());
    }

    // ── File output actually reaches the disk ───────────────────────────

    /// The core regression test for issue #50: a `LogConfig` with file output
    /// enabled must create the file and put the event in it.
    ///
    /// Installed with `tracing::subscriber::with_default` — a THREAD-LOCAL
    /// default — rather than `init_logging`, because the global subscriber
    /// slot is process-wide and claiming it here would silently disable every
    /// other test's logging (and could only ever be done by one test).
    #[test]
    fn file_output_creates_and_writes_the_log_file() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("nested").join("meedya-test.log");

        let config = LogConfig {
            level: "info".to_string(),
            console: false, // keep the test's own output clean
            file: true,
            file_path: Some(log_path.clone()),
            redact_pii: false,
            ..LogConfig::default()
        };

        // The file must not exist before the subscriber is built, otherwise
        // this test would pass on a stale artefact.
        assert!(!log_path.exists());

        let (subscriber, resolved) = build_subscriber(&config).unwrap();
        assert_eq!(resolved, Some(log_path.clone()));
        // The parent directory is created for the user — a fresh install has
        // no log directory yet.
        assert!(log_path.exists(), "log file was not created");

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(marker = "issue-50-file-sink", "hello from the file sink");
        });

        let contents = std::fs::read_to_string(&log_path).unwrap();
        assert!(
            contents.contains("hello from the file sink"),
            "log file did not receive the event: {contents:?}"
        );
        assert!(
            contents.contains("issue-50-file-sink"),
            "structured field missing from JSON record: {contents:?}"
        );
    }

    /// The file sink honours its own level: a `debug!` must not appear when
    /// the configured file level is `warn`.
    #[test]
    fn file_output_respects_configured_level() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("filtered.log");

        let config = LogConfig {
            level: "warn".to_string(),
            console: false,
            file: true,
            file_path: Some(log_path.clone()),
            redact_pii: false,
            ..LogConfig::default()
        };

        let (subscriber, _) = build_subscriber(&config).unwrap();
        tracing::subscriber::with_default(subscriber, || {
            tracing::debug!("this must be filtered out");
            tracing::warn!("this must be kept");
        });

        let contents = std::fs::read_to_string(&log_path).unwrap();
        assert!(!contents.contains("filtered out"), "level filter ignored");
        assert!(contents.contains("must be kept"));
    }

    /// Opening the log file twice appends rather than truncating — a second
    /// `meedya` invocation on the same day must not erase the first one's log.
    #[test]
    fn file_output_appends_across_runs() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("append.log");
        let config = LogConfig {
            console: false,
            file: true,
            file_path: Some(log_path.clone()),
            redact_pii: false,
            ..LogConfig::default()
        };

        for message in ["first run event", "second run event"] {
            let (subscriber, _) = build_subscriber(&config).unwrap();
            tracing::subscriber::with_default(subscriber, || tracing::info!("{message}"));
        }

        let contents = std::fs::read_to_string(&log_path).unwrap();
        assert!(contents.contains("first run event"));
        assert!(contents.contains("second run event"));
    }

    /// `open_log_file` refuses when file output is off, rather than silently
    /// inventing a path.
    #[test]
    fn open_log_file_requires_file_output_enabled() {
        let config = LogConfig::default();
        assert!(open_log_file(&config).is_err());
    }

    // ── PII redaction on the file sink ──────────────────────────────────

    /// A path under the real home directory must be reduced to `~/...` in the
    /// file, and the account name must not survive anywhere in the record.
    #[test]
    fn redaction_scrubs_home_directory_and_username_from_the_file() {
        let Some(home) = dirs::home_dir() else {
            // No home directory (an unusual CI sandbox) — nothing to redact.
            return;
        };
        let media_path = home.join("Music").join("track.mp3");

        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("redacted.log");
        let config = LogConfig {
            console: false,
            file: true,
            file_path: Some(log_path.clone()),
            redact_pii: true,
            ..LogConfig::default()
        };

        let (subscriber, _) = build_subscriber(&config).unwrap();
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(path = %media_path.display(), "processing media file");
        });

        let contents = std::fs::read_to_string(&log_path).unwrap();
        assert!(contents.contains("processing media file"));
        assert!(
            !contents.contains(home.to_string_lossy().as_ref()),
            "home directory leaked into the log: {contents:?}"
        );
        assert!(
            contents.contains('~'),
            "home was not replaced: {contents:?}"
        );
        // The filename itself is not PII and must still be diagnosable.
        assert!(contents.contains("track.mp3"));

        if let Some(username) = current_username()
            && username.chars().count() >= 3
        {
            assert!(
                !contents.contains(&username),
                "username {username:?} leaked into the log: {contents:?}"
            );
        }
    }

    /// With `redact_pii = false` the raw path is preserved — the switch has
    /// to actually switch something, or the test above proves nothing.
    #[test]
    fn redaction_disabled_leaves_the_path_intact() {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        let media_path = home.join("Music").join("track.mp3");

        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("raw.log");
        let config = LogConfig {
            console: false,
            file: true,
            file_path: Some(log_path.clone()),
            redact_pii: false,
            ..LogConfig::default()
        };

        let (subscriber, _) = build_subscriber(&config).unwrap();
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(path = %media_path.display(), "processing media file");
        });

        let contents = std::fs::read_to_string(&log_path).unwrap();
        assert!(
            contents.contains(home.to_string_lossy().as_ref()),
            "expected the un-redacted home path: {contents:?}"
        );
    }

    /// The record-level scrubber, exercised directly against a path built
    /// from the *current* username so the test cannot pass by accident on a
    /// hard-coded fixture.
    #[test]
    fn redact_log_record_rewrites_home_and_username() {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        let record = format!(
            "{{\"message\":\"scanning\",\"path\":\"{}/Music\"}}",
            home.display()
        );
        let redacted = redact_log_record(&record);
        assert!(redacted.contains("~/Music"), "got: {redacted}");
        assert!(!redacted.contains(home.to_string_lossy().as_ref()));
        assert!(redacted.contains("scanning"));
    }

    /// A record with no personal data must come through byte-identical.
    #[test]
    fn redact_log_record_leaves_unrelated_text_alone() {
        let record = "{\"message\":\"scanning\",\"path\":\"/opt/media/track.mp3\"}";
        assert_eq!(redact_log_record(record), record);
    }

    /// The writer must report the caller's byte count even when redaction
    /// shortens the record — a short count makes `write_all` re-send the
    /// tail and duplicates text in the log.
    #[test]
    fn redacting_writer_reports_full_length_after_shrinking() {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        if home.to_string_lossy().len() <= 1 {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("shrink.log");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .unwrap();

        let mut writer = RedactingFileWriter::new(file, true);
        let record = format!("{}/Music\n", home.display());
        // `write_all` would loop forever (or duplicate) on a short count.
        writer.write_all(record.as_bytes()).unwrap();
        writer.flush().unwrap();

        let contents = std::fs::read_to_string(&log_path).unwrap();
        assert_eq!(contents, "~/Music\n");
    }

    /// `RedactingFileWriter` is handed to `tracing_subscriber` as a
    /// `MakeWriter`; every handle it hands out must share one file, or
    /// concurrent events would overwrite each other.
    #[test]
    fn redacting_writer_handles_share_one_file() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("shared.log");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .unwrap();

        let writer = RedactingFileWriter::new(file, false);
        writer.make_writer().write_all(b"one\n").unwrap();
        writer.make_writer().write_all(b"two\n").unwrap();

        assert_eq!(std::fs::read_to_string(&log_path).unwrap(), "one\ntwo\n");
    }

    /// `Debug` is derived-by-hand; make sure it does not deadlock or panic.
    #[test]
    fn redacting_writer_debug_is_safe() {
        let dir = tempfile::tempdir().unwrap();
        let file = std::fs::File::create(dir.path().join("d.log")).unwrap();
        let writer = RedactingFileWriter::new(file, true);
        assert!(format!("{writer:?}").contains("redact"));
    }

    /// Before `init_logging` runs there is no active log file to report.
    /// (This crate's test binary never installs the global subscriber — see
    /// the comment on `file_output_creates_and_writes_the_log_file`.)
    #[test]
    fn active_log_file_is_none_without_init() {
        assert_eq!(active_log_file(), None);
    }
}
