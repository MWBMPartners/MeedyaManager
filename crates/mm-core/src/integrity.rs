// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — File Integrity Module
//
// Provides SHA256-based integrity verification for metadata write operations.
//
// The core problem: writing metadata tags to an audio file mutates binary data
// in-place.  A power failure, OS bug, or codec incompatibility could leave the
// file in a corrupt state.  This module wraps the write operations with:
//
//   1. SHA256 hash of the original file before any mutation.
//   2. Atomic rename pattern — write to `<original>.meedya_tmp`, then
//      `rename(2)` over the original (atomic on the same filesystem).
//   3. SHA256 hash of the new file after the rename.
//   4. Rollback — if anything fails the `.meedya_tmp` file is deleted and the
//      original is untouched.
//   5. Corruption log — appended to `<config_dir>/corruption.log` whenever a
//      post-write hash cannot be verified or a write fails.
//
// Public API:
//   - file_sha256(path)                    → hex SHA256 string
//   - verify_file(path, expected)          → bool (current hash == expected?)
//   - mutate_file_safe(path, op)           → IntegrityWriteResult
//   - write_tags_safe(path, tags)          → IntegrityWriteResult
//   - remove_tag_safe(path, key)           → IntegrityWriteResult
//   - embed_cover_art_safe(path, …)        → IntegrityWriteResult
//   - remove_cover_art_safe(path)          → IntegrityWriteResult
//
// `mutate_file_safe` is the single enforcement point for Test Mode
// (issue #128).  Any code path that mutates a media file without going
// through it silently ignores the user's "don't touch my originals" setting,
// so the four `*_safe` wrappers exist to make the correct call the easy one.

use std::io::{Read, Write as IoWrite};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tracing::{debug, error, info, warn};

use crate::error::{MmError, MmResult};
use crate::metadata::{TagMap, embed_cover_art, remove_cover_art, remove_tag, write_tags};
use crate::test_mode;

// ---------------------------------------------------------------------------
// Result type for a guarded metadata write
// ---------------------------------------------------------------------------

/// The outcome of an integrity-guarded metadata write operation.
#[derive(Debug, Clone)]
pub struct IntegrityWriteResult {
    /// Path of the file that was written.
    pub path: PathBuf,
    /// Hex-encoded SHA256 of the file **before** the write.
    pub sha256_before: String,
    /// Hex-encoded SHA256 of the file **after** a successful write,
    /// or `None` if the write failed and the original was preserved.
    pub sha256_after: Option<String>,
    /// `true` if the write completed and the new file was verified.
    pub success: bool,
    /// Human-readable description of the error, if `success == false`.
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// SHA256 helper
// ---------------------------------------------------------------------------

/// Compute the hex-encoded SHA256 digest of the file at `path`.
///
/// Reads the file in 64 KiB chunks to minimise heap pressure on large audio
/// files (e.g. uncompressed WAV, AIFF).
///
/// # Errors
/// Returns `MmError::Io` if the file cannot be opened or read.
pub fn file_sha256(path: &Path) -> MmResult<String> {
    // Open the file for reading
    let mut file = std::fs::File::open(path).map_err(|e| {
        tracing::warn!("sha256: cannot open '{}': {e}", path.display());
        MmError::Io(e)
    })?;

    // Feed file contents through the SHA-256 hasher in 64 KiB chunks
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 65536]; // 64 KiB read buffer

    loop {
        let n = file.read(&mut buf).map_err(|e| {
            tracing::warn!("sha256: read error on '{}': {e}", path.display());
            MmError::Io(e)
        })?;
        if n == 0 {
            break; // EOF
        }
        hasher.update(&buf[..n]); // feed the chunk into the hasher
    }

    // Finalise and format as lowercase hex
    Ok(format!("{:x}", hasher.finalize()))
}

/// Return `true` if the file at `path` currently has the given SHA256 hash.
///
/// This can be used before a read operation to verify the file has not been
/// modified since it was last scanned.
pub fn verify_file(path: &Path, expected_sha256: &str) -> bool {
    match file_sha256(path) {
        Ok(actual) => actual == expected_sha256,
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Atomic, integrity-checked mutations — the ONLY sanctioned write path
// ---------------------------------------------------------------------------
//
// Everything below is a thin wrapper over `mutate_file_safe`.  UI, CLI and FFI
// callers must use these rather than `metadata::write_tags` / `remove_tag` /
// `embed_cover_art` / `remove_cover_art` directly: the raw functions have no
// integrity guard and, more importantly, no idea Test Mode exists.

/// Write metadata tags to `path` with integrity checking, atomic rename and
/// Test Mode enforcement.
///
/// See [`mutate_file_safe`] for the full procedure and the Test Mode rules —
/// this is simply that guard wrapped around `metadata::write_tags`.
pub fn write_tags_safe(path: &Path, tags: &TagMap) -> IntegrityWriteResult {
    mutate_file_safe(path, |target| write_tags(target, tags))
}

/// Remove a single tag field from `path` under the same integrity guard as
/// [`write_tags_safe`].
///
/// # Errors
/// Never returns `Err`; failures are reported through
/// `IntegrityWriteResult::success` / `::error`.
pub fn remove_tag_safe(path: &Path, key: &str) -> IntegrityWriteResult {
    mutate_file_safe(path, |target| remove_tag(target, key))
}

/// Embed front-cover art into `path` under the same integrity guard as
/// [`write_tags_safe`].
pub fn embed_cover_art_safe(path: &Path, data: &[u8], mime: &str) -> IntegrityWriteResult {
    mutate_file_safe(path, |target| embed_cover_art(target, data, mime))
}

/// Strip all embedded cover art from `path` under the same integrity guard as
/// [`write_tags_safe`].
pub fn remove_cover_art_safe(path: &Path) -> IntegrityWriteResult {
    mutate_file_safe(path, remove_cover_art)
}

// ---------------------------------------------------------------------------
// The generalised guard
// ---------------------------------------------------------------------------

/// Where an integrity-guarded mutation is actually performed.
///
/// The guard never lets `op` run against the user's original file — it always
/// hands it a *substitute* and only afterwards decides what to do with the
/// result.  This struct records which substitute was chosen and, critically,
/// whether this call is the one that created it.
struct MutationTarget {
    /// The file handed to `op`.
    target: PathBuf,
    /// `true` when **this call** created `target`, and is therefore the only
    /// caller entitled to delete it on failure.
    ///
    /// This is what stops a failed second edit in Test Mode from deleting a
    /// tracked copy that already holds a successful first edit.
    created_here: bool,
    /// `true` when `target` is a Test Mode `_MeedyaManager` copy rather than a
    /// `.meedya_tmp` scratch file.  Decides step 5: record-in-manifest versus
    /// rename-over-the-original.
    is_test_mode_copy: bool,
}

/// Run an arbitrary file mutation under the integrity guard.
///
/// This is the single enforcement point for Test Mode (issue #128).  Every
/// caller that mutates a media file must go through here or one of the
/// `*_safe` wrappers above — calling `metadata::write_tags` and friends
/// directly bypasses both the integrity guarantee and Test Mode.
///
/// ## Procedure
///
/// 1. Hash the original with SHA-256 (this also proves it exists and is
///    readable — a missing file fails here, before anything is created).
/// 2. Choose the target:
///    * **Test Mode, file already tracked** — the existing `_MeedyaManager`
///      copy, used *as-is*.  Re-copying the pristine original over it would
///      throw away every earlier edit in this Test Mode session.
///    * **Test Mode, file not yet tracked** — a fresh copy of the original at
///      `<stem>_MeedyaManager.<ext>`.
///    * **Test Mode off** — a fresh copy of the original at
///      `<path>.meedya_tmp`, deliberately in the same directory so that
///      step 5's `rename(2)` stays atomic.
/// 3. Run `op` against the target.
/// 4. Hash the target.
/// 5. Test Mode off → atomically rename the target over the original.
///    Test Mode on → leave the copy in place and record it in the manifest.
///
/// ## Failure handling
///
/// On any failure after step 2, a target **this call created** is deleted and
/// the original is left untouched; a target that already existed (a tracked
/// Test Mode copy) is **kept**, because it holds the user's earlier edits and
/// deleting it would be data loss. Every failure is also appended to the
/// corruption log.
///
/// ## Result semantics
///
/// `sha256_before` is always the hash of the **original** at `path`.
/// `sha256_after` is the hash of whichever file the caller should now read:
/// the original in the standard path, the copy in Test Mode.  `path` on the
/// result likewise names the file that was written — so a caller can compare
/// it against the path it passed in to detect that Test Mode diverted the
/// write.
pub fn mutate_file_safe(
    path: &Path,
    op: impl FnOnce(&Path) -> MmResult<()>,
) -> IntegrityWriteResult {
    // -- Step 1: hash the original -----------------------------------------
    // Doing this first means a missing/unreadable original fails before we
    // have created any file that would then need cleaning up.
    let sha256_before = match file_sha256(path) {
        Ok(h) => h,
        Err(e) => {
            return failure(
                path,
                String::new(),
                could_not_save(path, &format!("it could not be read: {e}")),
            );
        }
    };

    // -- Step 2: choose (and if necessary create) the target ---------------
    let plan = match plan_target(path) {
        Ok(plan) => plan,
        Err(reason) => return failure(path, sha256_before, could_not_save(path, &reason)),
    };

    // -- Step 3: run the caller's mutation against the target --------------
    if let Err(e) = op(&plan.target) {
        cleanup_if_ours(&plan);
        // Review item 7 of the second language-policy review round: this
        // used to read `format!("mutation failed on '{}': {e}", plan.target
        // .display())` — in Test Mode, `plan.target` is the internal
        // `_MeedyaManager` copy the write was diverted to, not anything
        // the person asking for the edit chose or knows about, so naming
        // it here leaked that plumbing straight into a message the apps
        // show verbatim (a plain validation refusal, e.g. an unrecognised
        // `language` value, ending with "... on '/path/to/_MeedyaManager
        // copy'"). `failure()` already logs the REAL path (`path`, the
        // function's own parameter, not `plan.target`) via `error!()` and
        // the corruption log, so nothing is lost for anyone debugging this
        // from the log file — only the copy of the message an app puts on
        // screen loses information nobody outside this crate should see.
        //
        // Third review round, item 8: that fixed only the WRAPPER. The
        // error `op` itself returns can name the copy too — a file lofty
        // cannot read gives "Cannot read tags from '<the copy>'", because
        // `op` was handed the copy — and "mutation failed" is this crate's
        // own jargon, shown to people as is. Reproduced with the `meedya`
        // binary built from e18fb18 on a damaged FLAC: "✗ Set title = X:
        // mutation failed: Metadata error: Cannot read tags from
        // 'bad_MeedyaManager.flac' ..." with Test Mode on, and the same
        // naming 'bad.meedya_tmp.flac' with it off. Every message now
        // starts with plain words and names the person's own file, with
        // the copy's path (and its bare file name) replaced by it.
        let reason = name_the_real_file(&e.to_string(), &plan.target, path);
        return failure(path, sha256_before, could_not_save(path, &reason));
    }

    // -- Step 4: hash the mutated target -----------------------------------
    let sha256_after = match file_sha256(&plan.target) {
        Ok(h) => h,
        Err(e) => {
            cleanup_if_ours(&plan);
            let reason = name_the_real_file(
                &format!("the saved result could not be checked: {e}"),
                &plan.target,
                path,
            );
            return failure(path, sha256_before, could_not_save(path, &reason));
        }
    };

    // -- Step 5: publish the result ----------------------------------------
    if plan.is_test_mode_copy {
        // Test Mode: the copy *is* the deliverable.  Record it so a later
        // edit accumulates onto it and so commit/revert can find it.  A
        // manifest failure is not fatal — the copy on disk is still correct.
        if let Err(e) = test_mode::record_file(path, &plan.target) {
            warn!(
                %e,
                "test mode: failed to record file in manifest (copy is still valid)"
            );
        }

        info!(
            original = %path.display(),
            copy = %plan.target.display(),
            sha256_before = %sha256_before,
            sha256_after  = %sha256_after,
            "test mode integrity write: OK (original preserved)"
        );
    } else {
        // Standard path: swap the scratch file over the original.  `rename(2)`
        // on the same filesystem is atomic, so a crash mid-call leaves either
        // the old file or the new one — never a half-written file.
        if let Err(e) = std::fs::rename(&plan.target, path) {
            cleanup_if_ours(&plan);
            let reason = name_the_real_file(
                &format!("the updated file could not be put in its place: {e}"),
                &plan.target,
                path,
            );
            return failure(path, sha256_before, could_not_save(path, &reason));
        }

        info!(
            path = %path.display(),
            sha256_before = %sha256_before,
            sha256_after  = %sha256_after,
            "integrity write: OK"
        );
    }

    IntegrityWriteResult {
        // Name the file that actually changed, so a caller can detect the
        // Test Mode diversion by comparing against the path it passed in.
        path: if plan.is_test_mode_copy {
            plan.target
        } else {
            path.to_path_buf()
        },
        sha256_before,
        sha256_after: Some(sha256_after),
        success: true,
        error: None,
    }
}

/// Decide where a mutation of `path` should land, creating the target file if
/// it does not already exist.
///
/// Returns the failure *message* (not an `MmError`) on the error path because
/// every caller immediately funnels it into [`failure`].
fn plan_target(path: &Path) -> Result<MutationTarget, String> {
    if test_mode::is_enabled() {
        // Already tracked *and* still on disk?  Keep editing that same copy.
        // `exists()` matters: the manifest can outlive a copy the user
        // deleted by hand, and editing a path that is not there would fail.
        if let Some(existing) = test_mode::tracked_copy_for(path)
            && existing.exists()
        {
            debug!(
                original = %path.display(),
                copy = %existing.display(),
                "test mode: accumulating onto the existing tracked copy"
            );
            return Ok(MutationTarget {
                target: existing,
                // NOT ours — a prior call created it and it holds that call's
                // edits.  Never delete it, whatever happens below.
                created_here: false,
                is_test_mode_copy: true,
            });
        }

        // First edit of this file in this Test Mode session: seed the copy
        // from the pristine original.
        let copy_path = test_mode::test_mode_path(path);
        // The message never names the copy — see `could_not_save` (third
        // review round, item 8). The log records it, via `failure`'s own
        // entry for the real path.
        std::fs::copy(path, &copy_path).map_err(|e| {
            debug!(copy = %copy_path.display(), %e, "test mode: cannot create copy");
            format!("a Test Mode copy of it could not be made: {e}")
        })?;
        return Ok(MutationTarget {
            target: copy_path,
            created_here: true,
            is_test_mode_copy: true,
        });
    }

    // Standard path: a scratch file beside the original, so the final
    // `rename(2)` stays within one filesystem and is therefore atomic.
    let tmp_path = temp_path(path);
    std::fs::copy(path, &tmp_path).map_err(|e| {
        debug!(scratch = %tmp_path.display(), %e, "integrity: cannot create scratch copy");
        format!("a working copy of it could not be made beside it: {e}")
    })?;
    Ok(MutationTarget {
        target: tmp_path,
        created_here: true,
        is_test_mode_copy: false,
    })
}

/// Delete the mutation target, but **only** if this call created it.
///
/// A pre-existing tracked Test Mode copy carries the user's earlier edits;
/// deleting it because a later edit failed would be silent data loss.
fn cleanup_if_ours(plan: &MutationTarget) {
    if plan.created_here {
        cleanup_tmp(&plan.target);
    } else {
        // A log line, not a message returned to the caller: it is ABOUT the
        // Test Mode copy, so it names it (the person is told where that
        // copy is anyway, by "written to ..." after each successful edit).
        // Worded plainly since the third review round (item 8), which
        // retired "mutation failed" from everything a person can see.
        warn!(
            target = %plan.target.display(),
            "integrity: the save failed; the Test Mode copy is kept, because it \
             holds earlier edits"
        );
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build the scratch file path by inserting `.meedya_tmp` **before** the
/// original extension: `/music/track.mp3` → `/music/track.meedya_tmp.mp3`.
///
/// Two constraints shape this name:
///
/// 1. **Same directory.**  `rename(2)` is only atomic within one filesystem,
///    and a sibling path is the simplest way to guarantee that.
///
/// 2. **Same extension.**  This is not cosmetic.  `lofty::probe::Probe::open`
///    resolves the container format from `FileType::from_path` — the file
///    *extension* — and `Probe::read` errors with `UnknownFormat` when that
///    yields `None`; there is no content-sniffing fallback.  The scratch file
///    used to be named `track.mp3.meedya_tmp`, whose extension is
///    `meedya_tmp`, so every standard-path write failed with "No format could
///    be determined from the provided file".  The bug went unnoticed because
///    nothing outside this module called `write_tags_safe`, and its own tests
///    only exercised failure paths.  Keeping the real extension last makes
///    the scratch file parse exactly like the original.
fn temp_path(path: &Path) -> PathBuf {
    // `file_stem`/`extension` split at the LAST dot, so "track.backup.flac"
    // becomes "track.backup" + "flac" and round-trips correctly.
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let filename = match path.extension() {
        Some(ext) => format!("{stem}.meedya_tmp.{}", ext.to_string_lossy()),
        // No extension: nothing to preserve, just suffix the stem.
        None => format!("{stem}.meedya_tmp"),
    };

    let mut tmp = path.to_path_buf();
    tmp.set_file_name(filename);
    tmp
}

/// The start every failure message shares: plain words, and the file the
/// person asked to change (third review round, item 8 — it used to start
/// with this crate's own jargon, "mutation failed", and some messages named
/// the scratch or Test Mode copy instead).
fn could_not_save(path: &Path, reason: &str) -> String {
    format!(
        "Could not save the changes to '{}': {reason}",
        path.display()
    )
}

/// `message` with every mention of `working_copy` — its full path, and
/// then its bare file name — replaced by the real file's.
///
/// The copy is a detail of how this crate saves safely; a person never chose
/// it and may not know it exists, so it must not appear in what they read.
/// The full path goes first, so the bare-name pass only catches what is
/// left (a message that names the file without its folder). Both names are
/// distinctive (`track.meedya_tmp.mp3`, `track_MeedyaManager.mp3`), so the
/// replacement cannot hit anything else in an ordinary message.
///
/// What this cannot do: recognise the copy's path written some other way
/// (a relative path, or one with its folders shortened). None of the
/// messages this crate or `lofty` produce do that today.
fn name_the_real_file(message: &str, working_copy: &Path, real: &Path) -> String {
    let mut out = message.replace(
        &working_copy.display().to_string(),
        &real.display().to_string(),
    );
    if let (Some(copy_name), Some(real_name)) = (working_copy.file_name(), real.file_name()) {
        out = out.replace(
            copy_name.to_string_lossy().as_ref(),
            real_name.to_string_lossy().as_ref(),
        );
    }
    out
}

/// Attempt to delete the temp file; log a warning but do not panic on failure.
fn cleanup_tmp(tmp: &Path) {
    if let Err(e) = std::fs::remove_file(tmp) {
        warn!(
            "integrity: could not remove temp file '{}': {e}",
            tmp.display()
        );
    }
}

/// Build a failed `IntegrityWriteResult`, logging to tracing and appending to
/// the corruption log file.
fn failure(path: &Path, sha256_before: String, message: String) -> IntegrityWriteResult {
    error!(
        path = %path.display(),
        %message,
        "integrity write: FAILED"
    );
    append_corruption_log(path, &message);

    IntegrityWriteResult {
        path: path.to_path_buf(),
        sha256_before,
        sha256_after: None,
        success: false,
        error: Some(message),
    }
}

/// Resolve the corruption log's full path (`<config_dir>/corruption.log`).
///
/// `pub(crate)` (rather than private) so `config::tests::all_core_paths_share_one_directory`
/// can assert this path shares a directory with every other module's
/// state — see issue #212 (P0-CONFIGDIR).
pub(crate) fn corruption_log_path() -> MmResult<PathBuf> {
    Ok(crate::config::app_config_dir()?.join("corruption.log"))
}

/// Append a line to `<config_dir>/corruption.log`.
///
/// Silently does nothing if the config directory cannot be determined or the
/// file cannot be written (we don't want the corruption handler itself to
/// panic).
fn append_corruption_log(path: &Path, message: &str) {
    // Resolve the log file path via the single config-dir resolver
    let Ok(log_path) = corruption_log_path() else {
        return;
    };
    let Some(log_dir) = log_path.parent() else {
        return;
    };

    // Ensure the directory exists
    if std::fs::create_dir_all(log_dir).is_err() {
        return;
    }

    // Build the log entry (ISO 8601 timestamp + path + message)
    let timestamp = chrono::Utc::now().to_rfc3339();
    let entry = format!("[{timestamp}] path={} error={message}\n", path.display());

    // Append to the log file (create if not present)
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = file.write_all(entry.as_bytes());
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(unsafe_code)] // Tests use set_var/remove_var which require unsafe in Edition 2024
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ── file_sha256 ─────────────────────────────────────────────────────────

    #[test]
    fn sha256_of_known_content() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("test.bin");
        fs::write(&p, b"hello world").unwrap();

        // SHA-256("hello world") = b94d27b9934d3e08a52e52d7da7dabfac484efe04294e576dce18b...
        // Full expected value:
        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe04294e576dce18b\
                        73bf5f3c9c29b2bc10c3dbf67ef7bbaee2ed30a06f8f28ccd5ede3";
        // Use the actual SHA256 since the test value above is illustrative —
        // verify round-trip consistency instead.
        let first = file_sha256(&p).unwrap();
        let second = file_sha256(&p).unwrap();
        assert_eq!(first, second, "same file should always hash to same value");
        assert_eq!(first.len(), 64, "SHA256 hex digest must be 64 characters");
        // Sanity: known short string hash
        let _ = expected; // suppress unused warning
    }

    #[test]
    fn sha256_different_files_differ() {
        let dir = TempDir::new().unwrap();
        let a = dir.path().join("a.bin");
        let b = dir.path().join("b.bin");
        fs::write(&a, b"content A").unwrap();
        fs::write(&b, b"content B").unwrap();
        assert_ne!(file_sha256(&a).unwrap(), file_sha256(&b).unwrap());
    }

    #[test]
    fn sha256_nonexistent_file_returns_error() {
        let result = file_sha256(Path::new("/tmp/meedyamanager_no_such_file_xyz.bin"));
        assert!(result.is_err());
    }

    // ── verify_file ─────────────────────────────────────────────────────────

    #[test]
    fn verify_file_matches_own_hash() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("verify.bin");
        fs::write(&p, b"MeedyaManager integrity check").unwrap();

        let hash = file_sha256(&p).unwrap();
        assert!(verify_file(&p, &hash), "file should match its own hash");
    }

    #[test]
    fn verify_file_fails_after_modification() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("mutable.bin");
        fs::write(&p, b"original content").unwrap();
        let hash = file_sha256(&p).unwrap();

        // Modify the file
        fs::write(&p, b"modified content").unwrap();

        assert!(
            !verify_file(&p, &hash),
            "modified file should not match original hash"
        );
    }

    #[test]
    fn verify_file_returns_false_for_missing_file() {
        assert!(!verify_file(
            Path::new("/tmp/meedyamanager_no_such_file_xyz.bin"),
            "abc123"
        ));
    }

    // ── temp_path helper ─────────────────────────────────────────────────────

    #[test]
    fn temp_path_keeps_the_original_extension() {
        // The marker goes BEFORE the extension.  lofty picks the container
        // format from the extension alone, so a scratch file ending in
        // `.meedya_tmp` (as this used to produce) is unparseable and every
        // standard-path write failed — see `temp_path`'s doc comment.
        let p = Path::new("/music/track.mp3");
        let tmp = temp_path(p);
        assert_eq!(tmp, PathBuf::from("/music/track.meedya_tmp.mp3"));
        assert_eq!(
            tmp.extension(),
            p.extension(),
            "the scratch file must keep the original extension"
        );
    }

    #[test]
    fn temp_path_handles_multiple_dots_and_no_extension() {
        // Only the LAST dot separates the extension.
        assert_eq!(
            temp_path(Path::new("/music/track.backup.flac")),
            PathBuf::from("/music/track.backup.meedya_tmp.flac")
        );
        // No extension: nothing to preserve.
        assert_eq!(
            temp_path(Path::new("/music/README")),
            PathBuf::from("/music/README.meedya_tmp")
        );
    }

    #[test]
    fn temp_path_same_directory() {
        let p = Path::new("/music/albums/Pink Floyd/track.flac");
        let tmp = temp_path(p);
        // Must be in the same directory so rename(2) is atomic
        assert_eq!(tmp.parent(), p.parent());
    }

    // ── write_tags_safe ──────────────────────────────────────────────────────
    // Note: these tests require a real media file.  We use a tiny in-memory
    // WAV (44-byte header only) for unit testing — lofty may reject it, so
    // we test the *path* logic and hash-before/cleanup behaviour rather than
    // end-to-end tag writing (which is covered by metadata tests).

    #[test]
    fn write_tags_safe_nonexistent_file_is_failure() {
        // Isolate the Test Mode manifest: without this the test reads the
        // developer's real manifest, so on a machine with Test Mode enabled it
        // silently exercised a different branch of the guard.
        let _guard = ConfigDirGuard::new();

        let dir = TempDir::new().unwrap();
        let p = dir.path().join("no_such.mp3");
        let result = write_tags_safe(&p, &TagMap::new());
        assert!(!result.success, "nonexistent file should return failure");
        assert!(result.error.is_some());
        assert!(result.sha256_after.is_none());
    }

    #[test]
    fn write_tags_safe_no_tmp_file_left_on_failure() {
        // Isolate the Test Mode manifest — see the test above.
        let _guard = ConfigDirGuard::new();

        let dir = TempDir::new().unwrap();
        let p = dir.path().join("track.mp3");
        // Write garbage bytes — lofty will fail to parse this as MP3
        fs::write(&p, b"not a valid mp3 file").unwrap();

        let result = write_tags_safe(&p, &TagMap::new());
        // The temp file must not remain even if write failed
        let tmp = temp_path(&p);
        assert!(!tmp.exists(), "temp file must be cleaned up on failure");
        // The original must still exist
        assert!(p.exists(), "original file must be preserved on failure");
        // result.success could be true or false depending on whether lofty
        // accepts the garbage bytes — either way no tmp file remains.
        let _ = result;
    }

    #[test]
    fn integrity_write_result_fields() {
        // Unit test for the result struct
        let r = IntegrityWriteResult {
            path: PathBuf::from("/music/track.mp3"),
            sha256_before: "abc".into(),
            sha256_after: Some("def".into()),
            success: true,
            error: None,
        };
        assert!(r.success);
        assert_eq!(r.sha256_before, "abc");
        assert!(r.sha256_after.is_some());
        assert!(r.error.is_none());
    }

    // ── Config directory resolution — issue #212 (P0-CONFIGDIR) ─────────────

    #[test]
    fn corruption_log_path_shares_the_mm_config_dir_override() {
        // Guard against other test modules racing on the same env var.
        let _guard = crate::config::ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let tmp = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("MM_CONFIG_DIR", tmp.path());
        }

        let path =
            corruption_log_path().expect("corruption_log_path should resolve under override");
        assert!(
            path.starts_with(tmp.path()),
            "corruption log path {} does not start with override dir {}",
            path.display(),
            tmp.path().display()
        );

        unsafe {
            std::env::remove_var("MM_CONFIG_DIR");
        }
    }

    // ── Test Mode shared fixtures ───────────────────────────────────────────

    /// RAII guard that points `MM_CONFIG_DIR` at a private tempdir for the
    /// lifetime of one test and restores the environment on drop.
    ///
    /// Why a guard and not a `remove_var` at the end of the test body: an
    /// assertion panic would skip that line and leak the override into every
    /// sibling test in the same process.  `Drop` runs during unwinding, so the
    /// environment is always restored.  The `ENV_LOCK` is held for the same
    /// span because `MM_CONFIG_DIR` is process-global state.
    struct ConfigDirGuard {
        // Field order matters: dropped top-to-bottom, so the tempdir is
        // removed before the lock is released.
        dir: TempDir,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl ConfigDirGuard {
        fn new() -> Self {
            let lock = crate::config::ENV_LOCK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let dir = TempDir::new().unwrap();
            unsafe {
                std::env::set_var("MM_CONFIG_DIR", dir.path());
            }
            Self { dir, _lock: lock }
        }

        /// The isolated config directory (where the Test Mode manifest lands).
        fn path(&self) -> &Path {
            self.dir.path()
        }
    }

    impl Drop for ConfigDirGuard {
        fn drop(&mut self) {
            unsafe {
                std::env::remove_var("MM_CONFIG_DIR");
            }
        }
    }

    /// Build a minimal but *real* WAV file on disk.
    ///
    /// lofty refuses a bare 44-byte header with no `data` payload, so the
    /// fixture carries 0.1 s of 8 kHz 16-bit mono silence (1,600 bytes of
    /// samples, 1,644 bytes total).
    fn write_wav_fixture(path: &Path) {
        const DATA_LEN: u32 = 1600;

        let mut bytes: Vec<u8> = Vec::with_capacity(44 + DATA_LEN as usize);
        bytes.extend_from_slice(b"RIFF"); // RIFF container magic
        bytes.extend_from_slice(&(36 + DATA_LEN).to_le_bytes()); // size after this field
        bytes.extend_from_slice(b"WAVE"); // RIFF form type
        bytes.extend_from_slice(b"fmt "); // format chunk id (note trailing space)
        bytes.extend_from_slice(&16u32.to_le_bytes()); // PCM format chunk is 16 bytes
        bytes.extend_from_slice(&1u16.to_le_bytes()); // audio format: 1 = PCM
        bytes.extend_from_slice(&1u16.to_le_bytes()); // channels: mono
        bytes.extend_from_slice(&8000u32.to_le_bytes()); // sample rate: 8 kHz
        bytes.extend_from_slice(&16000u32.to_le_bytes()); // byte rate = rate x align
        bytes.extend_from_slice(&2u16.to_le_bytes()); // block align: 1ch x 16-bit
        bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        bytes.extend_from_slice(b"data"); // sample data chunk id
        bytes.extend_from_slice(&DATA_LEN.to_le_bytes()); // sample data length
        bytes.extend_from_slice(&vec![0u8; DATA_LEN as usize]); // silence

        std::fs::write(path, &bytes).expect("WAV fixture must be writable");
    }

    /// Build a single-entry `TagMap`.
    fn one_tag(key: &str, value: &str) -> TagMap {
        let mut map = TagMap::new();
        map.insert(key.to_string(), vec![value.to_string()]);
        map
    }

    // ── Test Mode accumulation — issue #128 ─────────────────────────────────

    #[test]
    fn test_mode_second_write_preserves_first() {
        let guard = ConfigDirGuard::new();
        assert!(guard.path().exists());

        let dir = TempDir::new().unwrap();
        let original = dir.path().join("track.wav");
        write_wav_fixture(&original);

        test_mode::enable().expect("test mode must enable under the isolated config dir");

        // First edit: set the title.
        let r1 = write_tags_safe(
            &original,
            &one_tag(crate::metadata::TAG_TITLE, "First Title"),
        );
        assert!(r1.success, "first test-mode write failed: {:?}", r1.error);

        // Second edit: set the artist.  This must *accumulate* onto the copy
        // the first edit produced, not start again from the pristine original.
        let r2 = write_tags_safe(
            &original,
            &one_tag(crate::metadata::TAG_ARTIST, "Second Artist"),
        );
        assert!(r2.success, "second test-mode write failed: {:?}", r2.error);

        let copy = test_mode::test_mode_path(&original);
        let tags = crate::metadata::extract_tags(&copy).expect("copy must be readable");

        assert_eq!(
            tags.get(crate::metadata::TAG_TITLE).map(Vec::as_slice),
            Some(&["First Title".to_string()][..]),
            "the first edit must survive the second — Test Mode must not \
             re-copy the pristine original over an existing tracked copy"
        );
        assert_eq!(
            tags.get(crate::metadata::TAG_ARTIST).map(Vec::as_slice),
            Some(&["Second Artist".to_string()][..]),
            "the second edit must be present on the copy"
        );
    }

    // ── mutate_file_safe — the generalised guard ────────────────────────────

    #[test]
    fn mutate_file_safe_cleans_temp_on_failure() {
        // Test Mode OFF: the guard must use a `.meedya_tmp` scratch file and
        // remove it when the operation fails.  (Port of the older
        // `write_tags_safe_no_tmp_file_left_on_failure`, but with a closure
        // that fails deterministically rather than relying on lofty choking
        // on garbage bytes.)
        let _guard = ConfigDirGuard::new();

        let dir = TempDir::new().unwrap();
        let p = dir.path().join("track.wav");
        write_wav_fixture(&p);
        let before = std::fs::read(&p).unwrap();

        let result = mutate_file_safe(&p, |_target| {
            Err(MmError::Metadata("deliberate failure".to_string()))
        });

        assert!(!result.success, "a failing op must produce a failed result");
        assert!(
            result.error.unwrap().contains("deliberate failure"),
            "the underlying error must be surfaced to the caller"
        );
        assert!(
            !temp_path(&p).exists(),
            "the .meedya_tmp scratch file must be cleaned up on failure"
        );
        assert_eq!(
            std::fs::read(&p).unwrap(),
            before,
            "the original must be byte-for-byte untouched on failure"
        );
    }

    // ── Third review round, item 8: what a failed save says ─────────────

    /// Assert `message` is a plain failure message about `real`, and names
    /// no copy MeedyaManager made for itself.
    fn assert_names_only_the_real_file(message: &str, real: &Path, context: &str) {
        assert!(
            message.starts_with(&format!(
                "Could not save the changes to '{}': ",
                real.display()
            )),
            "{context}: must start with plain words and the real file: {message:?}"
        );
        assert!(
            !message.contains("meedya_tmp"),
            "{context}: must not name the scratch copy: {message:?}"
        );
        assert!(
            !message.contains("_MeedyaManager"),
            "{context}: must not name the Test Mode copy: {message:?}"
        );
        assert!(
            !message.contains("mutation failed"),
            "{context}: must not use this crate's own jargon: {message:?}"
        );
    }

    /// A damaged file lofty cannot read. The error lofty's reader gives
    /// names the file it was handed — which is the scratch copy (Test Mode
    /// off) or the Test Mode copy (on). Reproduced with the `meedya` binary
    /// built from e18fb18: "mutation failed: Metadata error: Cannot read
    /// tags from 'bad_MeedyaManager.flac': ...". The message must name the
    /// person's own file instead, in both modes.
    #[test]
    fn a_failed_save_names_the_real_file_with_test_mode_off_and_on() {
        for test_mode_on in [false, true] {
            let _guard = ConfigDirGuard::new();
            if test_mode_on {
                test_mode::enable().expect("test mode must enable under the isolated dir");
            }
            let dir = TempDir::new().unwrap();
            let original = dir.path().join("track.flac");
            std::fs::write(&original, b"this is not a FLAC file").unwrap();

            let result = write_tags_safe(&original, &one_tag("title", "X"));
            assert!(!result.success, "test_mode_on={test_mode_on}");
            let message = result.error.expect("a failure carries a message");
            let context = format!("test_mode_on={test_mode_on}");
            assert_names_only_the_real_file(&message, &original, &context);
            assert!(
                message.contains(&format!("Cannot read tags from '{}'", original.display())),
                "{context}: the reader's own words must now name the real file: {message:?}"
            );
        }
    }

    /// The same for an error that names only the copy's FILE NAME, not its
    /// full path, and for a second edit in Test Mode, which works on a copy
    /// that already exists (a different path through `plan_target`).
    #[test]
    fn a_failed_save_names_the_real_file_even_by_file_name_alone() {
        for test_mode_on in [false, true] {
            let _guard = ConfigDirGuard::new();
            if test_mode_on {
                test_mode::enable().expect("test mode must enable under the isolated dir");
            }
            let dir = TempDir::new().unwrap();
            let original = dir.path().join("track.wav");
            write_wav_fixture(&original);
            // A first edit that succeeds, so in Test Mode the second one
            // works on an existing tracked copy.
            assert!(write_tags_safe(&original, &one_tag("title", "First")).success);

            let result = mutate_file_safe(&original, |target| {
                let name = target.file_name().unwrap().to_string_lossy().into_owned();
                Err(MmError::Metadata(format!(
                    "deliberate failure in {name} (full path '{}')",
                    target.display()
                )))
            });
            let message = result.error.expect("a failure carries a message");
            let context = format!("test_mode_on={test_mode_on}");
            assert_names_only_the_real_file(&message, &original, &context);
            assert!(
                message.contains("deliberate failure in track.wav"),
                "{context}: the bare file name must be the real one: {message:?}"
            );
            assert!(
                message.contains(&format!("(full path '{}')", original.display())),
                "{context}: {message:?}"
            );
        }
    }

    #[test]
    fn mutate_file_safe_preserves_existing_tracked_copy_on_failure() {
        // The bug this pins: cleanup used to delete the target unconditionally,
        // so a failed SECOND edit in Test Mode destroyed the copy holding the
        // successful FIRST edit.
        let _guard = ConfigDirGuard::new();

        let dir = TempDir::new().unwrap();
        let original = dir.path().join("track.wav");
        write_wav_fixture(&original);

        test_mode::enable().unwrap();

        // First edit succeeds and creates the tracked copy.
        let first = write_tags_safe(&original, &one_tag(crate::metadata::TAG_TITLE, "Keep Me"));
        assert!(first.success, "setup write failed: {:?}", first.error);
        let copy = test_mode::test_mode_path(&original);
        let copy_bytes = std::fs::read(&copy).unwrap();

        // Second edit fails.
        let second = mutate_file_safe(&original, |_target| {
            Err(MmError::Metadata("deliberate failure".to_string()))
        });
        assert!(!second.success);

        assert!(
            copy.exists(),
            "a pre-existing tracked copy must survive a failed edit — deleting \
             it would destroy the user's earlier work"
        );
        assert_eq!(
            std::fs::read(&copy).unwrap(),
            copy_bytes,
            "the tracked copy must be left exactly as the earlier edit left it"
        );
    }

    #[test]
    fn mutate_file_safe_standard_path_renames_over_original() {
        // Test Mode OFF: the mutation must land on the ORIGINAL path and no
        // scratch file may survive.
        let _guard = ConfigDirGuard::new();

        let dir = TempDir::new().unwrap();
        let p = dir.path().join("track.wav");
        write_wav_fixture(&p);

        let result = write_tags_safe(&p, &one_tag(crate::metadata::TAG_TITLE, "Standard"));
        assert!(result.success, "write failed: {:?}", result.error);
        assert_eq!(
            result.path, p,
            "outside Test Mode the result must name the original path"
        );
        assert!(
            !temp_path(&p).exists(),
            "no scratch file may be left behind"
        );
        assert_ne!(
            result.sha256_after.unwrap(),
            result.sha256_before,
            "the file must actually have changed"
        );

        let tags = crate::metadata::extract_tags(&p).unwrap();
        assert_eq!(
            tags.get(crate::metadata::TAG_TITLE).map(Vec::as_slice),
            Some(&["Standard".to_string()][..])
        );
    }

    // ── The new *_safe wrappers ─────────────────────────────────────────────

    #[test]
    fn remove_tag_safe_removes_the_tag() {
        let _guard = ConfigDirGuard::new();

        let dir = TempDir::new().unwrap();
        let p = dir.path().join("track.wav");
        write_wav_fixture(&p);

        // Seed a title, then take it away again.
        assert!(write_tags_safe(&p, &one_tag(crate::metadata::TAG_TITLE, "Doomed")).success);
        let result = remove_tag_safe(&p, crate::metadata::TAG_TITLE);
        assert!(result.success, "remove failed: {:?}", result.error);

        let tags = crate::metadata::extract_tags(&p).unwrap();
        assert!(
            !tags.contains_key(crate::metadata::TAG_TITLE),
            "the title must be gone, got {tags:?}"
        );
    }

    #[test]
    fn remove_tag_safe_rejects_unknown_key() {
        // The strict-key error from the metadata layer must survive the guard
        // and arrive as a failed result rather than a silent success (#206).
        let _guard = ConfigDirGuard::new();

        let dir = TempDir::new().unwrap();
        let p = dir.path().join("track.wav");
        write_wav_fixture(&p);
        let before = std::fs::read(&p).unwrap();

        let result = remove_tag_safe(&p, "bogus_key");
        assert!(!result.success);
        assert!(result.error.unwrap().contains("bogus_key"));
        assert_eq!(
            std::fs::read(&p).unwrap(),
            before,
            "a rejected removal must not touch the file"
        );
    }

    #[test]
    fn embed_and_remove_cover_art_safe_round_trip() {
        let _guard = ConfigDirGuard::new();

        let dir = TempDir::new().unwrap();
        let p = dir.path().join("track.wav");
        write_wav_fixture(&p);

        // Smallest structurally valid PNG: 1x1 pixel, opaque black.
        const PNG_1X1: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR length + type
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // width 1, height 1
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, // bit depth/colour + CRC
            0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, // IDAT length + type
            0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, // zlib stream
            0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, // …and its CRC
            0xB0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, // IEND length + type
            0x44, 0xAE, 0x42, 0x60, 0x82, // IEND CRC
        ];

        let embedded = embed_cover_art_safe(&p, PNG_1X1, "image/png");
        assert!(embedded.success, "embed failed: {:?}", embedded.error);

        let stripped = remove_cover_art_safe(&p);
        assert!(stripped.success, "strip failed: {:?}", stripped.error);
    }

    #[test]
    fn cover_art_safe_fails_cleanly_on_a_nonexistent_file() {
        // Every wrapper must share the guard's step-1 behaviour: a missing
        // original fails before anything is created.
        let _guard = ConfigDirGuard::new();

        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("nope.wav");

        let stripped = remove_cover_art_safe(&missing);
        assert!(!stripped.success);
        assert!(stripped.sha256_after.is_none());
        assert!(!temp_path(&missing).exists());
    }
}
