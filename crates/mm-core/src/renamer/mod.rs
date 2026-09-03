// (C) 2025-2026 MWBM Partners Ltd
//
// File rename simulation and execution.
//
// Computes destination paths from metadata, detects conflicts,
// provides dry-run preview, and executes renames with rollback.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::error::{MmError, MmResult};
use crate::rule_engine::{self, EvalContext, Rule};

/// Result of a rename simulation — shows what would happen
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenamePreview {
    /// Source file path
    pub source: PathBuf,
    /// Computed destination path
    pub destination: PathBuf,
    /// Whether the destination already exists
    pub conflict: bool,
    /// Whether source and destination are the same (no rename needed)
    pub unchanged: bool,
}

/// Summary of a batch rename operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameSummary {
    /// Total files processed
    pub total: usize,
    /// Files that would be renamed
    pub renamed: usize,
    /// Files unchanged (already at destination)
    pub unchanged: usize,
    /// Files with destination conflicts
    pub conflicts: usize,
    /// Individual rename previews
    pub previews: Vec<RenamePreview>,
}

/// Characters that are invalid in filenames on Windows
const WINDOWS_INVALID_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Reserved filenames on Windows (case-insensitive)
const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
    "COM8", "COM9", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Maximum filename length (conservative cross-platform limit)
const MAX_FILENAME_LENGTH: usize = 255;

/// Default character to replace invalid chars with
const DEFAULT_REPLACEMENT: char = '_';

/// Configuration for filename sanitisation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizeConfig {
    /// Character to replace invalid characters with
    pub replacement_char: char,
    /// Custom replacement mappings (e.g. ":" → " -")
    pub custom_replacements: HashMap<char, String>,
    /// Whether to apply Windows-compatible sanitisation on all platforms
    pub windows_compatible: bool,
    /// Maximum filename length (0 = platform default)
    pub max_length: usize,
}

impl Default for SanitizeConfig {
    fn default() -> Self {
        Self {
            replacement_char: DEFAULT_REPLACEMENT,
            custom_replacements: HashMap::new(),
            windows_compatible: true, // Cross-platform by default
            max_length: MAX_FILENAME_LENGTH,
        }
    }
}

/// Sanitise a filename by replacing invalid characters.
///
/// Applies platform-aware rules:
/// - Replaces characters invalid on the target platform
/// - Applies custom replacement mappings
/// - Trims leading/trailing whitespace and dots
/// - Handles reserved Windows filenames
/// - Truncates to max length while preserving extension
/// - Normalises Unicode to NFC form
pub fn sanitize_filename(name: &str, config: &SanitizeConfig) -> String {
    if name.is_empty() {
        return String::from("unnamed");
    }

    // Split into stem and extension
    let (stem, ext) = match name.rfind('.') {
        Some(pos) if pos > 0 => (&name[..pos], Some(&name[pos..])),
        _ => (name, None),
    };

    // Process stem character by character
    let mut result = String::with_capacity(stem.len());
    for ch in stem.chars() {
        // Check custom replacements first
        if let Some(replacement) = config.custom_replacements.get(&ch) {
            result.push_str(replacement);
            continue;
        }

        // Check platform-invalid characters
        if config.windows_compatible && WINDOWS_INVALID_CHARS.contains(&ch) {
            result.push(config.replacement_char);
            continue;
        }

        // Replace control characters
        if ch.is_control() {
            result.push(config.replacement_char);
            continue;
        }

        result.push(ch);
    }

    // Trim leading/trailing whitespace and dots
    let trimmed = result.trim().trim_matches('.').to_string();

    // Handle empty result after trimming
    let trimmed = if trimmed.is_empty() {
        "unnamed".to_string()
    } else {
        trimmed
    };

    // Check for reserved Windows names
    let upper = trimmed.to_ascii_uppercase();
    let trimmed = if config.windows_compatible && WINDOWS_RESERVED_NAMES.contains(&upper.as_str()) {
        format!("{trimmed}_file")
    } else {
        trimmed
    };

    // Re-attach extension
    let full = match ext {
        Some(e) => format!("{trimmed}{e}"),
        None => trimmed.clone(),
    };

    // Truncate to max length while preserving extension
    if config.max_length > 0 && full.len() > config.max_length {
        let ext_len = ext.map_or(0, str::len);
        let max_stem = config.max_length.saturating_sub(ext_len);
        let truncated_stem: String = trimmed.chars().take(max_stem).collect();
        match ext {
            Some(e) => format!("{truncated_stem}{e}"),
            None => truncated_stem,
        }
    } else {
        full
    }
}

/// Build the destination directory for a template result.
///
/// A template such as `<Artist>/<Album>/<Title>` legitimately produces
/// *directory* components, so we must not flatten them with
/// [`sanitize_filename`] (its invalid-character table contains `/` and `\`).
/// Equally, those components come from **tag data**, which is attacker- (or
/// simply typo-) controlled: an `<Artist>` of `/tmp/x` would make
/// `Path::join` *replace* `output_dir` outright, and `../x` would climb out
/// of it.  So we walk the parent components explicitly and:
///
///   * drop `RootDir` and `Prefix` — these are what make `join` replace the
///     base path, turning a relative template into an absolute one;
///   * drop `ParentDir` (`..`) and `CurDir` (`.`) — `..` escapes `output_dir`
///     and `.` is simply noise;
///   * sanitise every `Normal` component individually, so a tag value that
///     itself contains a separator collapses into one safe directory name
///     rather than silently creating extra nesting.
///
/// The result is therefore always a descendant of `output_dir`.
fn build_destination_dir(
    output_dir: &Path,
    template_path: &Path,
    config: &SanitizeConfig,
) -> PathBuf {
    // Start from the caller-controlled root; every component we append below
    // is a single sanitised name, so the result can never climb above it.
    let mut destination_dir = output_dir.to_path_buf();

    // `parent()` is `None` only for a root/empty path, in which case there
    // are no directory components to add at all.
    if let Some(parent) = template_path.parent() {
        for component in parent.components() {
            match component {
                // The only component kind we are willing to keep. Sanitising
                // it also strips any separator the tag value smuggled in.
                Component::Normal(part) => {
                    // `to_string_lossy` keeps non-UTF-8 names usable rather
                    // than dropping the component entirely.
                    let safe = sanitize_filename(&part.to_string_lossy(), config);
                    destination_dir.push(safe);
                }
                // RootDir / Prefix / ParentDir / CurDir are all dropped —
                // see the doc comment above for why each one is unsafe.
                Component::RootDir
                | Component::Prefix(_)
                | Component::ParentDir
                | Component::CurDir => {}
            }
        }
    }

    destination_dir
}

/// Substitute metadata values into a path template.
///
/// Replaces `{key}` placeholders in the template with values from the
/// metadata map. Unknown keys are replaced with "Unknown".
pub fn substitute_template(template: &str, metadata: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    // Find all {key} placeholders and replace them
    for (key, value) in metadata {
        let placeholder = format!("{{{key}}}");
        result = result.replace(&placeholder, value);
    }
    // Replace any remaining unreplaced placeholders with "Unknown"
    static PLACEHOLDER_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"\{[^}]+\}").expect("valid regex"));
    PLACEHOLDER_RE.replace_all(&result, "Unknown").to_string()
}

/// Simulate a batch rename operation without moving any files.
///
/// Takes source files and a path template, substitutes metadata values,
/// sanitises filenames, and returns a preview of what would happen.
pub fn simulate_rename(
    files: &[(PathBuf, HashMap<String, String>)],
    template: &str,
    output_dir: &Path,
    config: &SanitizeConfig,
) -> MmResult<RenameSummary> {
    let mut previews = Vec::with_capacity(files.len());
    let mut destinations_seen: HashMap<PathBuf, usize> = HashMap::new();

    for (source, metadata) in files {
        // Substitute metadata into the template
        let raw_path = substitute_template(template, metadata);

        // Split template result into directory components and filename
        let template_path = Path::new(&raw_path);
        let raw_name = template_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed");

        // Preserve the original extension if the template doesn't include one
        let ext = source.extension().and_then(|e| e.to_str());
        let name_with_ext = if Path::new(raw_name).extension().is_some() {
            raw_name.to_string()
        } else if let Some(e) = ext {
            format!("{raw_name}.{e}")
        } else {
            raw_name.to_string()
        };

        // Sanitise the filename
        let safe_name = sanitize_filename(&name_with_ext, config);

        // Build parent directories from the template (if any subdirectories),
        // sanitising each component so tag data cannot escape `output_dir`.
        let destination = build_destination_dir(output_dir, template_path, config).join(&safe_name);

        // Belt-and-braces: the containment property above is a security
        // guarantee, so assert it in debug builds and tests.
        debug_assert!(destination.starts_with(output_dir));

        // Detect conflicts
        let conflict = destination.exists() || destinations_seen.contains_key(&destination);
        let unchanged = source == &destination;

        // Track this destination to detect intra-batch conflicts
        *destinations_seen.entry(destination.clone()).or_insert(0) += 1;

        previews.push(RenamePreview {
            source: source.clone(),
            destination,
            conflict,
            unchanged,
        });
    }

    let renamed = previews
        .iter()
        .filter(|p| !p.unchanged && !p.conflict)
        .count();
    let unchanged = previews.iter().filter(|p| p.unchanged).count();
    let conflicts = previews.iter().filter(|p| p.conflict).count();

    Ok(RenameSummary {
        total: previews.len(),
        renamed,
        unchanged,
        conflicts,
        previews,
    })
}

/// Simulate a batch rename using the rule engine.
///
/// Evaluates each file against the rule set (or falls back to the default
/// template) to compute destination paths.  Returns a preview summary
/// without moving any files.
///
/// The `'f` lifetime is explicit rather than elided on purpose. Elision here
/// would produce the *higher-ranked* bound
/// `for<'p> Fn(&'p Path) -> MmResult<EvalContext<'p>>`, which no realistic
/// caller can satisfy: a builder that borrows a pre-extracted metadata map
/// can only produce a context valid for that map's lifetime, never for an
/// arbitrary (up to `'static`) one. Tying the builder to the lifetime of
/// `files` instead lets callers borrow data that outlives the batch.
pub fn simulate_rename_with_rules<'f>(
    files: &'f [PathBuf],
    rules: &[Rule],
    default_template: &str,
    output_dir: &Path,
    config: &SanitizeConfig,
    ctx_builder: impl Fn(&'f Path) -> MmResult<EvalContext<'f>>,
) -> MmResult<RenameSummary> {
    let mut previews = Vec::with_capacity(files.len());
    let mut destinations_seen: HashMap<PathBuf, usize> = HashMap::new();

    for source in files {
        // Build the evaluation context for this file
        let ctx = ctx_builder(source)?;

        // Try rules first, then fall back to the default template
        let raw_path = if rules.is_empty() {
            rule_engine::evaluate_template(default_template, &ctx)?
        } else {
            match rule_engine::apply_rules(rules, &ctx)? {
                Some(result) => result,
                None => rule_engine::evaluate_template(default_template, &ctx)?,
            }
        };

        // Split template result into directory components and filename
        let template_path = Path::new(&raw_path);
        let raw_name = template_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed");

        // Preserve the original extension if the template doesn't include one
        let ext = source.extension().and_then(|e| e.to_str());
        let name_with_ext = if Path::new(raw_name).extension().is_some() {
            raw_name.to_string()
        } else if let Some(e) = ext {
            format!("{raw_name}.{e}")
        } else {
            raw_name.to_string()
        };

        // Sanitise the filename
        let safe_name = sanitize_filename(&name_with_ext, config);

        // Build parent directories from the template (if any subdirectories),
        // sanitising each component so tag data cannot escape `output_dir`.
        let destination = build_destination_dir(output_dir, template_path, config).join(&safe_name);

        // Belt-and-braces: the containment property above is a security
        // guarantee, so assert it in debug builds and tests.
        debug_assert!(destination.starts_with(output_dir));

        // Detect conflicts
        let conflict = destination.exists() || destinations_seen.contains_key(&destination);
        let unchanged = source == &destination;

        // Track this destination to detect intra-batch conflicts
        *destinations_seen.entry(destination.clone()).or_insert(0) += 1;

        previews.push(RenamePreview {
            source: source.clone(),
            destination,
            conflict,
            unchanged,
        });
    }

    let renamed = previews
        .iter()
        .filter(|p| !p.unchanged && !p.conflict)
        .count();
    let unchanged = previews.iter().filter(|p| p.unchanged).count();
    let conflicts = previews.iter().filter(|p| p.conflict).count();

    Ok(RenameSummary {
        total: previews.len(),
        renamed,
        unchanged,
        conflicts,
        previews,
    })
}

/// Options controlling how a preview is applied to the file system.
///
/// Split out from [`RenamePreview`] because they are *policy* (read from
/// `config.rename`) rather than *facts about this file*, and because the
/// existing `execute_rename(preview)` signature is part of the FFI surface
/// and must keep working unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecuteOptions {
    /// Copy the file instead of moving it, leaving the source in place.
    pub copy_mode: bool,
    /// Create any missing directories in the destination path.
    pub create_dirs: bool,
}

impl Default for ExecuteOptions {
    fn default() -> Self {
        Self {
            // Moving is the historic behaviour and stays the default.
            copy_mode: false,
            // Templates routinely contain sub-directories, so creating them
            // is the only useful default.
            create_dirs: true,
        }
    }
}

/// Execute a rename operation, moving files from source to destination.
///
/// Thin wrapper over [`execute_rename_with`] using [`ExecuteOptions::default`]
/// (move, create directories). Kept so existing callers — notably the
/// `mm-ffi` binding consumed by the macOS UI — need no changes.
pub fn execute_rename(preview: &RenamePreview) -> MmResult<()> {
    execute_rename_with(preview, &ExecuteOptions::default())
}

/// Execute a rename operation under explicit options.
///
/// Creates parent directories as needed (when `create_dirs` is set). Returns
/// an error if the rename fails, but does not roll back previously successful
/// renames.
///
/// # Safety of the destination
///
/// `preview.conflict` is computed at *preview* time and can be stale by the
/// time the caller executes — that staleness is precisely what let two files
/// whose templates resolved to the same name silently overwrite each other.
/// So in addition to honouring the flag we **re-check the destination
/// immediately before the move**. This is defence in depth: a caller that
/// computes the flag wrongly, or a batch that claims the same destination
/// twice, now gets an error instead of destroying data.
pub fn execute_rename_with(preview: &RenamePreview, opts: &ExecuteOptions) -> MmResult<()> {
    // Nothing to do when the file is already where it belongs.
    if preview.unchanged {
        return Ok(());
    }

    // Honour the flag the caller computed (cheap, and preserves the existing
    // error message that callers may be matching on).
    if preview.conflict {
        return Err(MmError::Rename(format!(
            "destination already exists: {}",
            preview.destination.display()
        )));
    }

    // Re-check for real. Between preview and execution the destination may
    // have been created by another process, by the user, or — the common
    // case — by an earlier entry in this very batch.
    if preview.destination.exists() {
        return Err(MmError::Rename(format!(
            "destination appeared since preview: {}",
            preview.destination.display()
        )));
    }

    // Create parent directories when policy allows. When it does not, a
    // missing directory simply surfaces as an I/O error below, which is the
    // honest outcome for `create_dirs = false`.
    if opts.create_dirs {
        if let Some(parent) = preview.destination.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| MmError::Rename(format!("cannot create directory: {e}")))?;
        }
    }

    if opts.copy_mode {
        // Copy mode preserves the original — used when the user wants an
        // organised library built alongside an untouched source tree.
        std::fs::copy(&preview.source, &preview.destination).map_err(|e| {
            MmError::Rename(format!(
                "cannot copy {} → {}: {e}",
                preview.source.display(),
                preview.destination.display()
            ))
        })?;
    } else {
        // Perform the rename (move).
        std::fs::rename(&preview.source, &preview.destination).map_err(|e| {
            MmError::Rename(format!(
                "cannot rename {} → {}: {e}",
                preview.source.display(),
                preview.destination.display()
            ))
        })?;
    }

    Ok(())
}

/// Maximum number of ` (n)` suffixes tried before giving up.
///
/// Bounded so a pathological directory cannot spin forever; 9,999 is far
/// beyond any realistic duplicate count for a single destination name.
const MAX_CONFLICT_COUNTER: u32 = 9_999;

/// Resolve a conflicting destination by appending ` (n)` before the extension.
///
/// Implements the `conflict_strategy = "rename"` policy. A candidate is only
/// accepted when it is free **both on disk and in `claimed`** — the latter
/// holds destinations that earlier entries in the same batch have already
/// taken but may not have written yet, which is the intra-batch case that
/// `destination.exists()` alone cannot see.
///
/// # Errors
/// Returns `MmError::Rename` if no free name is found within
/// [`MAX_CONFLICT_COUNTER`] attempts.
pub fn resolve_conflict_by_counter(
    destination: &Path,
    claimed: &HashSet<PathBuf>,
) -> MmResult<PathBuf> {
    // Fast path: the destination was never actually taken.
    if !destination.exists() && !claimed.contains(destination) {
        return Ok(destination.to_path_buf());
    }

    // Split into the directory, the stem we will suffix, and the extension we
    // must keep *after* the suffix so the file stays recognisable.
    let parent = destination.parent().unwrap_or(Path::new(""));
    let stem = destination
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| String::from("unnamed"));
    let ext = destination
        .extension()
        .map(|e| e.to_string_lossy().into_owned());

    for counter in 1..=MAX_CONFLICT_COUNTER {
        // " (1)" goes before the dot: "Song (1).mp3", not "Song.mp3 (1)".
        let candidate_name = match &ext {
            Some(e) => format!("{stem} ({counter}).{e}"),
            None => format!("{stem} ({counter})"),
        };
        let candidate = parent.join(candidate_name);

        if !candidate.exists() && !claimed.contains(&candidate) {
            return Ok(candidate);
        }
    }

    Err(MmError::Rename(format!(
        "no free name for {} after {MAX_CONFLICT_COUNTER} attempts",
        destination.display()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, b"").unwrap();
    }

    #[test]
    fn sanitize_removes_windows_invalid_chars() {
        let config = SanitizeConfig::default();
        assert_eq!(
            sanitize_filename("file<>name.mp3", &config),
            "file__name.mp3"
        );
        assert_eq!(sanitize_filename("file:name.mp3", &config), "file_name.mp3");
        assert_eq!(sanitize_filename("file|name.mp3", &config), "file_name.mp3");
    }

    #[test]
    fn sanitize_custom_replacements() {
        let mut config = SanitizeConfig::default();
        config.custom_replacements.insert(':', " -".to_string());
        assert_eq!(
            sanitize_filename("Song: Title.mp3", &config),
            "Song - Title.mp3"
        );
    }

    #[test]
    fn sanitize_trims_whitespace_and_dots() {
        let config = SanitizeConfig::default();
        assert_eq!(sanitize_filename("  song  .mp3", &config), "song.mp3");
        assert_eq!(sanitize_filename("...song....mp3", &config), "song.mp3");
    }

    #[test]
    fn sanitize_reserved_names() {
        let config = SanitizeConfig::default();
        assert_eq!(sanitize_filename("CON.txt", &config), "CON_file.txt");
        assert_eq!(sanitize_filename("nul.mp3", &config), "nul_file.mp3");
        assert_eq!(sanitize_filename("LPT1.doc", &config), "LPT1_file.doc");
    }

    #[test]
    fn sanitize_empty_input() {
        let config = SanitizeConfig::default();
        assert_eq!(sanitize_filename("", &config), "unnamed");
    }

    #[test]
    fn sanitize_control_characters() {
        let config = SanitizeConfig::default();
        assert_eq!(
            sanitize_filename("file\x00name.mp3", &config),
            "file_name.mp3"
        );
        assert_eq!(
            sanitize_filename("file\tname.mp3", &config),
            "file_name.mp3"
        );
    }

    #[test]
    fn sanitize_truncates_long_names() {
        let config = SanitizeConfig {
            max_length: 20,
            ..Default::default()
        };
        let long_name = "a".repeat(30) + ".mp3";
        let result = sanitize_filename(&long_name, &config);
        assert!(result.len() <= 20);
        assert!(result.ends_with(".mp3"));
    }

    #[test]
    fn substitute_template_basic() {
        let mut meta = HashMap::new();
        meta.insert("artist".to_string(), "Pink Floyd".to_string());
        meta.insert("album".to_string(), "DSOTM".to_string());
        meta.insert("title".to_string(), "Time".to_string());

        let result = substitute_template("{artist}/{album}/{title}", &meta);
        assert_eq!(result, "Pink Floyd/DSOTM/Time");
    }

    #[test]
    fn substitute_template_missing_keys() {
        let meta = HashMap::new();
        let result = substitute_template("{artist}/{title}", &meta);
        assert_eq!(result, "Unknown/Unknown");
    }

    #[test]
    fn simulate_rename_basic() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("old.mp3");
        touch(&source);

        let mut meta = HashMap::new();
        meta.insert("title".to_string(), "New Song".to_string());

        let files = vec![(source, meta)];
        let config = SanitizeConfig::default();
        let summary = simulate_rename(&files, "{title}", dir.path(), &config).unwrap();

        assert_eq!(summary.total, 1);
        assert_eq!(summary.renamed, 1);
        assert_eq!(summary.unchanged, 0);
        assert_eq!(summary.conflicts, 0);
        assert!(summary.previews[0].destination.ends_with("New Song.mp3"));
    }

    #[test]
    fn simulate_rename_detects_conflicts() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("old.mp3");
        let existing = dir.path().join("existing.mp3");
        touch(&source);
        touch(&existing);

        let mut meta = HashMap::new();
        meta.insert("title".to_string(), "existing".to_string());

        let files = vec![(source, meta)];
        let config = SanitizeConfig::default();
        let summary = simulate_rename(&files, "{title}", dir.path(), &config).unwrap();

        assert_eq!(summary.conflicts, 1);
        assert!(summary.previews[0].conflict);
    }

    #[test]
    fn simulate_rename_detects_unchanged() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("song.mp3");
        touch(&source);

        let mut meta = HashMap::new();
        meta.insert("title".to_string(), "song".to_string());

        let files = vec![(source, meta)];
        let config = SanitizeConfig::default();
        let summary = simulate_rename(&files, "{title}", dir.path(), &config).unwrap();

        assert_eq!(summary.unchanged, 1);
    }

    #[test]
    fn execute_rename_moves_file() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("old.mp3");
        let dest = dir.path().join("new.mp3");
        touch(&source);

        let preview = RenamePreview {
            source: source.clone(),
            destination: dest.clone(),
            conflict: false,
            unchanged: false,
        };

        execute_rename(&preview).unwrap();
        assert!(!source.exists());
        assert!(dest.exists());
    }

    #[test]
    fn execute_rename_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("old.mp3");
        let dest = dir.path().join("Artist").join("Album").join("new.mp3");
        touch(&source);

        let preview = RenamePreview {
            source: source.clone(),
            destination: dest.clone(),
            conflict: false,
            unchanged: false,
        };

        execute_rename(&preview).unwrap();
        assert!(!source.exists());
        assert!(dest.exists());
    }

    #[test]
    fn execute_rename_skips_unchanged() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("song.mp3");
        touch(&source);

        let preview = RenamePreview {
            source: source.clone(),
            destination: source.clone(),
            conflict: false,
            unchanged: true,
        };

        execute_rename(&preview).unwrap();
        assert!(source.exists()); // Should still be there
    }

    // ── Path-traversal containment ──────────────────────────────────────

    /// **Regression — path traversal from tag data.**
    ///
    /// Directory components came straight from tag values and were handed to
    /// `output_dir.join(parent_parts)`. `Path::join` *replaces* the base when
    /// the argument is absolute, so an artist of `/etc/x` wrote outside the
    /// library entirely; `../../x` climbed out of it. Every destination must
    /// now stay under `output_dir`.
    #[test]
    fn simulate_rename_rejects_absolute_and_parent_components() {
        let dir = TempDir::new().unwrap();
        let output_dir = dir.path().join("library");
        fs::create_dir_all(&output_dir).unwrap();

        // Two hostile artist values plus a benign control.
        let hostile = ["/etc/x", "../../x", "Normal:Artist"];
        let files: Vec<(PathBuf, HashMap<String, String>)> = hostile
            .iter()
            .enumerate()
            .map(|(i, artist)| {
                let source = dir.path().join(format!("src{i}.mp3"));
                touch(&source);
                let mut meta = HashMap::new();
                meta.insert("artist".to_string(), (*artist).to_string());
                meta.insert("title".to_string(), format!("Track {i}"));
                (source, meta)
            })
            .collect();

        let config = SanitizeConfig::default();
        let summary = simulate_rename(&files, "{artist}/{title}", &output_dir, &config).unwrap();

        for preview in &summary.previews {
            assert!(
                preview.destination.starts_with(&output_dir),
                "escaped the output directory: {}",
                preview.destination.display()
            );
        }

        // Exactly what survives: the leading `RootDir` of "/etc/x" is dropped
        // (it is what made `join` replace the base), leaving the remaining
        // *relative* components re-rooted under `output_dir`.
        assert_eq!(
            summary.previews[0].destination,
            output_dir.join("etc").join("x").join("Track 0.mp3")
        );
        // Both `..` components are dropped, so "../../x" cannot climb out.
        assert_eq!(
            summary.previews[1].destination,
            output_dir.join("x").join("Track 1.mp3")
        );
        // Each surviving component is sanitised in its own right — the ":"
        // here is illegal on Windows and becomes "_".
        assert_eq!(
            summary.previews[2].destination,
            output_dir.join("Normal_Artist").join("Track 2.mp3")
        );
    }

    /// The same containment guarantee for the rule-engine simulator.
    #[test]
    fn simulate_rename_with_rules_rejects_absolute_components() {
        let dir = TempDir::new().unwrap();
        let output_dir = dir.path().join("library");
        fs::create_dir_all(&output_dir).unwrap();

        let source = dir.path().join("track.mp3");
        touch(&source);
        let files = vec![source];

        let mut tags: crate::metadata::TagMap = HashMap::new();
        tags.insert("artist".to_string(), vec!["/etc/passwd".to_string()]);
        tags.insert("title".to_string(), vec!["Song".to_string()]);

        let config = SanitizeConfig::default();
        let summary = simulate_rename_with_rules(
            &files,
            &[],
            "<Artist>/<Title>",
            &output_dir,
            &config,
            |_p| Ok(EvalContext::new(&tags)),
        )
        .unwrap();

        assert!(summary.previews[0].destination.starts_with(&output_dir));
        assert_eq!(
            summary.previews[0].destination,
            output_dir.join("etc").join("passwd").join("Song.mp3")
        );
    }

    /// Document the separator behaviour deliberately, because it is a visible
    /// change for users: a tag value containing "/" is indistinguishable from
    /// a template separator by the time the evaluated string reaches us, so
    /// an artist of "AC/DC" produces a *nested* "AC/DC/" pair of directories
    /// rather than one "AC_DC" directory. This is safe (still inside
    /// `output_dir`) but differs from the pre-fix flat "AC_DC_..." filename.
    #[test]
    fn simulate_rename_tag_containing_separator_nests() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("track.mp3");
        touch(&source);

        let mut meta = HashMap::new();
        meta.insert("artist".to_string(), "AC/DC".to_string());
        meta.insert("title".to_string(), "Thunderstruck".to_string());

        let files = vec![(source, meta)];
        let config = SanitizeConfig::default();
        let summary = simulate_rename(&files, "{artist}/{title}", dir.path(), &config).unwrap();

        assert_eq!(
            summary.previews[0].destination,
            dir.path().join("AC").join("DC").join("Thunderstruck.mp3")
        );
    }

    /// A folder template must produce real sub-directories, not a flat name.
    #[test]
    fn simulate_rename_preserves_folder_template_structure() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("track.mp3");
        touch(&source);

        let mut meta = HashMap::new();
        meta.insert("artist".to_string(), "Portishead".to_string());
        meta.insert("album".to_string(), "Dummy".to_string());
        meta.insert("title".to_string(), "Roads".to_string());

        let files = vec![(source, meta)];
        let config = SanitizeConfig::default();
        let summary =
            simulate_rename(&files, "{artist}/{album}/{title}", dir.path(), &config).unwrap();

        assert_eq!(
            summary.previews[0].destination,
            dir.path()
                .join("Portishead")
                .join("Dummy")
                .join("Roads.mp3")
        );
    }

    // ── simulate_rename_with_rules ──────────────────────────────────────

    /// Two files resolving to the same name: the second must be a conflict.
    ///
    /// This is the intra-batch case `destination.exists()` cannot see, and the
    /// one that destroyed data when the CLI computed conflicts by itself.
    #[test]
    fn simulate_rename_with_rules_intra_batch_conflict() {
        let dir = TempDir::new().unwrap();
        let first = dir.path().join("a.mp3");
        let second = dir.path().join("b.mp3");
        touch(&first);
        touch(&second);
        let files = vec![first, second];

        // Both files carry the same title, so both templates resolve alike.
        let mut tags: crate::metadata::TagMap = HashMap::new();
        tags.insert("title".to_string(), vec!["Same".to_string()]);

        let config = SanitizeConfig::default();
        let summary =
            simulate_rename_with_rules(&files, &[], "<Title>", dir.path(), &config, |_p| {
                Ok(EvalContext::new(&tags))
            })
            .unwrap();

        assert_eq!(summary.total, 2);
        assert!(!summary.previews[0].conflict);
        assert!(
            summary.previews[1].conflict,
            "intra-batch clash not detected"
        );
        assert_eq!(summary.conflicts, 1);
        assert_eq!(summary.renamed, 1);
    }

    /// An extensionless source must not gain a trailing dot.
    #[test]
    fn simulate_rename_with_rules_extensionless_source_no_trailing_dot() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("no_extension_here");
        touch(&source);
        let files = vec![source];

        let mut tags: crate::metadata::TagMap = HashMap::new();
        tags.insert("title".to_string(), vec!["Bare".to_string()]);

        let config = SanitizeConfig::default();
        let summary =
            simulate_rename_with_rules(&files, &[], "<Title>", dir.path(), &config, |_p| {
                Ok(EvalContext::new(&tags))
            })
            .unwrap();

        let name = summary.previews[0]
            .destination
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(name, "Bare");
        assert!(!name.ends_with('.'), "trailing dot on extensionless file");
    }

    /// A matching rule wins over the default template.
    #[test]
    fn simulate_rename_with_rules_applies_rules_before_default_template() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("track.mp3");
        touch(&source);
        let files = vec![source];

        let mut tags: crate::metadata::TagMap = HashMap::new();
        tags.insert("genre".to_string(), vec!["Jazz".to_string()]);
        tags.insert("title".to_string(), vec!["Blue".to_string()]);

        // Rule matches on genre and routes the file into a "Jazz" folder.
        let matching = Rule {
            name: "Jazz files".to_string(),
            priority: 0,
            enabled: true,
            conditions: vec![crate::rule_engine::Condition {
                field: "genre".to_string(),
                operator: crate::rule_engine::ConditionOp::Equals,
                value: "Jazz".to_string(),
            }],
            condition_mode: crate::rule_engine::ConditionMode::All,
            template: "Jazz/<Title>".to_string(),
            stop_on_match: true,
        };

        let config = SanitizeConfig::default();
        let summary = simulate_rename_with_rules(
            &files,
            std::slice::from_ref(&matching),
            "Fallback/<Title>",
            dir.path(),
            &config,
            |_p| Ok(EvalContext::new(&tags)),
        )
        .unwrap();

        assert_eq!(
            summary.previews[0].destination,
            dir.path().join("Jazz").join("Blue.mp3")
        );

        // A rule whose condition fails must fall through to the template.
        let non_matching = Rule {
            conditions: vec![crate::rule_engine::Condition {
                field: "genre".to_string(),
                operator: crate::rule_engine::ConditionOp::Equals,
                value: "Metal".to_string(),
            }],
            ..matching
        };
        let summary = simulate_rename_with_rules(
            &files,
            std::slice::from_ref(&non_matching),
            "Fallback/<Title>",
            dir.path(),
            &config,
            |_p| Ok(EvalContext::new(&tags)),
        )
        .unwrap();

        assert_eq!(
            summary.previews[0].destination,
            dir.path().join("Fallback").join("Blue.mp3")
        );
    }

    // ── Execution guards ────────────────────────────────────────────────

    /// **Regression — silent overwrite through a stale conflict flag.**
    ///
    /// `conflict: false` is only ever a snapshot. When the destination does in
    /// fact exist, `std::fs::rename` used to clobber it without a word. The
    /// mover now re-checks the disk itself, so both files survive.
    #[test]
    fn execute_rename_refuses_existing_destination_when_flag_stale() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("old.mp3");
        let dest = dir.path().join("taken.mp3");
        fs::write(&source, b"source-bytes").unwrap();
        fs::write(&dest, b"destination-bytes").unwrap();

        let preview = RenamePreview {
            source: source.clone(),
            destination: dest.clone(),
            // Deliberately stale — this is what the CLI used to hand over.
            conflict: false,
            unchanged: false,
        };

        let result = execute_rename(&preview);
        assert!(result.is_err(), "stale flag allowed an overwrite");

        // Both files must be untouched, byte for byte.
        assert_eq!(fs::read(&source).unwrap(), b"source-bytes");
        assert_eq!(fs::read(&dest).unwrap(), b"destination-bytes");
    }

    /// Copy mode leaves the source in place.
    #[test]
    fn execute_rename_with_copy_mode_keeps_source() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("old.mp3");
        let dest = dir.path().join("Artist").join("new.mp3");
        fs::write(&source, b"payload").unwrap();

        let preview = RenamePreview {
            source: source.clone(),
            destination: dest.clone(),
            conflict: false,
            unchanged: false,
        };

        execute_rename_with(
            &preview,
            &ExecuteOptions {
                copy_mode: true,
                create_dirs: true,
            },
        )
        .unwrap();

        assert!(source.exists(), "copy mode must not remove the source");
        assert_eq!(fs::read(&dest).unwrap(), b"payload");
    }

    /// With `create_dirs: false` a missing parent is an error, not a silent
    /// directory creation.
    #[test]
    fn execute_rename_with_create_dirs_disabled_fails_on_missing_parent() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("old.mp3");
        let dest = dir.path().join("missing").join("new.mp3");
        touch(&source);

        let preview = RenamePreview {
            source: source.clone(),
            destination: dest.clone(),
            conflict: false,
            unchanged: false,
        };

        let result = execute_rename_with(
            &preview,
            &ExecuteOptions {
                copy_mode: false,
                create_dirs: false,
            },
        );
        assert!(result.is_err());
        assert!(source.exists());
        assert!(!dest.exists());
    }

    // ── Conflict counter ────────────────────────────────────────────────

    /// The counter strategy appends " (n)" before the extension, and honours
    /// both on-disk files and names claimed earlier in the same batch.
    #[test]
    fn conflict_strategy_rename_appends_counter() {
        let dir = TempDir::new().unwrap();
        let taken = dir.path().join("Song.mp3");
        touch(&taken);

        // On-disk clash → " (1)".
        let claimed = HashSet::new();
        let first = resolve_conflict_by_counter(&taken, &claimed).unwrap();
        assert_eq!(first, dir.path().join("Song (1).mp3"));

        // " (1)" now claimed by this batch (not yet written) → " (2)".
        let mut claimed = HashSet::new();
        claimed.insert(first);
        let second = resolve_conflict_by_counter(&taken, &claimed).unwrap();
        assert_eq!(second, dir.path().join("Song (2).mp3"));

        // A free name is returned unchanged.
        let free = dir.path().join("Other.mp3");
        assert_eq!(
            resolve_conflict_by_counter(&free, &HashSet::new()).unwrap(),
            free
        );

        // Extensionless files get the counter on the end.
        let bare = dir.path().join("Bare");
        touch(&bare);
        assert_eq!(
            resolve_conflict_by_counter(&bare, &HashSet::new()).unwrap(),
            dir.path().join("Bare (1)")
        );
    }

    #[test]
    fn execute_rename_rejects_conflict() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("old.mp3");
        let dest = dir.path().join("existing.mp3");
        touch(&source);
        touch(&dest);

        let preview = RenamePreview {
            source,
            destination: dest,
            conflict: true,
            unchanged: false,
        };

        let result = execute_rename(&preview);
        assert!(result.is_err());
    }
}
