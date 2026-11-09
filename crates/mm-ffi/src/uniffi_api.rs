// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — UniFFI-exported API functions
//
// All public functions are annotated with `#[uniffi::export]` and exposed
// to Swift (macOS) via the UniFFI proc-macro scaffolding registered in lib.rs.
//
// Design rules:
//   - All parameters and return types must be UniFFI-compatible
//   - TagMap (HashMap<String, Vec<String>>) is flattened to Vec<TagEntry>
//     because UniFFI does not support nested generic types
//   - Errors are converted from MmError → MmFfiError at every boundary
//   - The file watcher uses a background thread to forward channel events
//     to the UniFFI callback interface

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use mm_core::classify;
use mm_core::config::AppConfig;
// The integrity guard — the only sanctioned way to mutate a media file.
use mm_core::integrity;
use mm_core::metadata::{self, TagMap};
use mm_core::renamer::{self, SanitizeConfig};
use mm_core::rule_engine::{
    self,
    evaluator::{EvalContext, evaluate_template},
};
use mm_core::watcher::{self, WatchEvent, WatcherConfig};

use crate::callbacks::WatchCallback;
use crate::types::{
    AudioPropertiesFfi, MmFfiError, RenamePreviewFfi, TagEntry, ValidationResult, WatchEventFfi,
    WriteMetadataResult,
};

// ---------------------------------------------------------------------------
// Version
// ---------------------------------------------------------------------------

/// Return the MeedyaManager core version string (e.g. "0.5.0").
#[uniffi::export]
pub fn mm_version() -> String {
    // Injected at compile time from Cargo.toml [package].version
    env!("CARGO_PKG_VERSION").to_string()
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Return the platform-specific path to `settings.json5`.
///
/// macOS:   `~/Library/Application Support/MeedyaManager/settings.json5`
/// Linux:   `~/.config/MeedyaManager/settings.json5`
/// Windows: `%APPDATA%\MeedyaManager\settings.json5`
#[uniffi::export]
pub fn config_path() -> String {
    // Routed through `mm_core::config::app_config_dir()` (the single
    // resolver — see issue #212, P0-CONFIGDIR) rather than calling
    // `dirs::config_dir()` directly, so this always matches where
    // `AppConfig::load()` actually reads from.
    mm_core::config::app_config_dir()
        .map_or_else(
            |_| PathBuf::from("settings.json5"),
            |d| d.join("settings.json5"),
        )
        .to_string_lossy()
        .into_owned()
}

/// Load the configuration from the platform-default location.
///
/// Returns the config serialized as a JSON string for the Settings panel.
/// If no config file exists, returns the default configuration JSON.
#[uniffi::export]
pub fn config_load() -> Result<String, MmFfiError> {
    // AppConfig::load() reads from the platform config dir (no arguments)
    let config = AppConfig::load().map_err(MmFfiError::from)?;

    serde_json::to_string_pretty(&config).map_err(|e| MmFfiError::Config(e.to_string()))
}

// ---------------------------------------------------------------------------
// Media scanning & rename preview
// ---------------------------------------------------------------------------

/// Scan a directory and compute rename previews for all media files.
///
/// - `directory` — absolute path to scan
/// - `template`  — MusicBee-style rename template (e.g. `"<Artist> - <Title>"`)
/// - `recursive` — if true, descend into sub-directories
///
/// Returns previews sorted by source path. The UI shows this list before the
/// user confirms execution via `execute_renames`.
#[uniffi::export]
pub fn scan_directory(
    directory: String,
    template: String,
    recursive: bool,
) -> Result<Vec<RenamePreviewFfi>, MmFfiError> {
    let dir_path = PathBuf::from(&directory);

    // Collect paths of all recognised media files in the directory
    let media_files =
        collect_media_files(&dir_path, recursive).map_err(|e| MmFfiError::Io(e.to_string()))?;

    // For each file: read metadata, flatten to HashMap<String, String>, collect
    let files_with_tags: Vec<(PathBuf, HashMap<String, String>)> = media_files
        .into_iter()
        .map(|path| {
            // Read tags; use empty map for files that cannot be read
            let flat = metadata::extract_tags(&path)
                .map(flatten_tag_map)
                .unwrap_or_default();
            (path, flat)
        })
        .collect();

    if files_with_tags.is_empty() {
        return Ok(vec![]);
    }

    // Use the source directory itself as the output directory
    // (renamer computes relative names, UI confirms full paths)
    let sanitize_cfg = SanitizeConfig::default();

    // Simulate renames using the renamer module
    let summary = renamer::simulate_rename(&files_with_tags, &template, &dir_path, &sanitize_cfg)
        .map_err(MmFfiError::from)?;

    // Convert mm-core RenamePreview to FFI-safe RenamePreviewFfi
    let mut previews: Vec<RenamePreviewFfi> = summary
        .previews
        .into_iter()
        .map(RenamePreviewFfi::from_core)
        .collect();

    // Sort by source path for deterministic UI display order
    previews.sort_by(|a, b| a.source.cmp(&b.source));

    Ok(previews)
}

/// The purpose text recorded in the write lock's info note while this
/// function holds it — see `mm_core::state::LockFile::try_acquire`.
const LOCK_PURPOSE: &str = "the MeedyaManager app moving files";

/// Execute a set of renames (non-conflicting, non-unchanged only).
///
/// Returns the count of files successfully renamed.
///
/// Routed through [`renamer::execute_rename`] rather than moving the file
/// directly via the standard library (issue #201 reached the CLI, mm-core and
/// mm-gtk but missed this FFI path). That gives the FFI surface the same
/// safety net every other caller gets: a re-check of the destination
/// immediately before the move (a preview's `conflict` flag can go stale
/// between scan and execute — another process, the user, or an earlier
/// entry in this very batch may have created it since), plus the
/// create-directories and copy-vs-move policy handling — see
/// [`renamer::ExecuteOptions`].
#[uniffi::export]
pub fn execute_renames(previews: Vec<RenamePreviewFfi>) -> Result<u32, MmFfiError> {
    // Take the write lock before moving anything (issue #49).
    //
    // **Correction (issue #49, review round):** this comment used to say
    // "the desktop apps reach the mover through this one function" — true
    // only of the macOS app. The Windows app moves files itself without
    // ever calling `execute_renames`, and the Linux (GTK) app calls
    // mm-core's per-file `renamer::execute_rename` directly; neither of
    // those two takes this lock at all yet (#226). So today this guard only
    // actually protects the macOS app against colliding with a
    // `meedya scan --execute` already running in a terminal — it is not yet
    // the single choke point every platform goes through.
    //
    // `_write_lock` stays alive for the whole function, so it is given back
    // the moment we return, however we return — see `mm_core::state`'s
    // module docs for what "given back" means now (the file is never
    // deleted; only the operating system's lock on it is released).
    //
    // `try_acquire_default` distinguishes "somebody already holds it" (an
    // `Ok(None)`, the ordinary busy case) from "the lock could not even be
    // attempted" (an `Err`, e.g. a settings folder on a network drive that
    // does not support file locking) — both are turned into text by the
    // shared functions in `mm_core::state` (issue #49, review round: this
    // used to build its own separate wording here, naming no holder and
    // saying "or stop it" — stopping a copy of MeedyaManager part-way
    // through a batch of renames can leave a library half-moved, which is
    // exactly why that advice was wrong and the CLI's own wording never
    // included it). Both still map to `MmFfiError::Rename` rather than a
    // new enum case, because adding one would change the generated Swift
    // bindings and require every caller to be rebuilt.
    let write_lock = mm_core::state::LockFile::try_acquire_default(LOCK_PURPOSE)
        .map_err(|e| MmFfiError::Rename(mm_core::state::lock_unavailable_message(&e)))?;

    let Some(_write_lock) = write_lock else {
        // Busy — read who holds it, best effort, purely to tell the user
        // something useful; `holder` never decides anything, the refusal
        // above already came straight from the operating system's own lock.
        let holder = mm_core::state::LockFile::holder(&mm_core::state::LockFile::default_path());
        return Err(MmFfiError::Rename(mm_core::state::lock_busy_message(
            holder.as_ref(),
        )));
    };

    let mut count = 0u32;

    for preview in previews {
        // Skip unchanged files and conflicting destinations
        if preview.unchanged || preview.conflict {
            continue;
        }

        // Rebuild the mm-core RenamePreview the FFI-safe type was flattened
        // from, so `execute_rename` gets exactly the facts it re-validates.
        let core_preview = renamer::RenamePreview {
            source: PathBuf::from(&preview.source),
            destination: PathBuf::from(&preview.destination),
            conflict: preview.conflict,
            unchanged: preview.unchanged,
        };

        renamer::execute_rename(&core_preview).map_err(MmFfiError::from)?;

        count += 1;
    }

    Ok(count)
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

/// Read all metadata tags from a single media file.
///
/// Returns a list of `TagEntry` pairs sorted by key for stable UI display.
/// Multi-value tags (e.g. multiple artists) are joined with "; ".
///
/// The `language` entry's `note` is set when the file's tags disagree about
/// the language (see `TagEntry::note`) — for the app to show beside the
/// value, so the other answer is not hidden. Every other entry's `note` is
/// `None`.
#[uniffi::export]
pub fn get_metadata(path: String) -> Result<Vec<TagEntry>, MmFfiError> {
    let file_path = PathBuf::from(&path);

    // Extract the multi-value TagMap from the file
    let tag_map = metadata::extract_tags(&file_path).map_err(MmFfiError::from)?;

    // Third review round of the language-policy work, item 3: `tag_map`
    // shows ONE language even when the file's tags hold two different ones
    // (a tag that can hold the full code is read first — TRACK-070). This
    // reads the file a second time, only to find that out; it cannot fail
    // the call, because it is a report, not the data.
    let language_note = metadata::language::disagreement_note(&file_path);

    // Flatten: join Vec<String> values with "; " and build sorted TagEntry list
    let mut entries: Vec<TagEntry> = tag_map
        .into_iter()
        .map(|(key, values)| TagEntry {
            note: if key == metadata::TAG_LANGUAGE {
                language_note.clone()
            } else {
                None
            },
            key,
            // Join multi-values with the canonical MeedyaManager delimiter
            value: values.join("; "),
        })
        .collect();

    // Sort by key for deterministic display order
    entries.sort_by(|a, b| a.key.cmp(&b.key));

    Ok(entries)
}

/// Write/update metadata tags on a media file.
///
/// Only the tags in `tags` are written; existing tags not in the list are
/// preserved. Multi-value tags can be passed with "; " as the delimiter.
///
/// ## Behaviour changes callers must know about
///
/// * **Test Mode is honoured.**  With Test Mode enabled the original file is
///   left byte-for-byte untouched and the tags land on a `_MeedyaManager`
///   copy beside it (issue #128).  A subsequent write accumulates onto that
///   same copy.
/// * **Unknown keys are rejected.**  A key with no `ItemKey` mapping used to
///   be dropped silently, so the call returned `Ok` having changed nothing.
///   It now returns `MmFfiError::Metadata` naming the key and listing the
///   valid ones (issue #206).  Use `mm_core::metadata::known_tag_keys` — or
///   simply write back keys obtained from `get_metadata`.
/// * **`"language"` follows the shared MWBM-MEDIA-LANG policy.**  A value
///   nothing recognises as a language (a full BCP 47 tag such as `"en-GB"`,
///   or an old three-letter code such as `"fre"`, are both accepted) is
///   refused with `MmFfiError::Metadata`, before anything is written — the
///   same all-or-nothing guarantee an unknown key already has.  What is
///   actually written differs by tag format (an ID3 tag — an MP3's, or one
///   embedded in a WAV — can only ever hold the old three-letter form;
///   every other format keeps the full value) — see
///   `mm_core::metadata::language` and
///   `docs/standards/media-language-bcp47-policy.md`.  A `"language"` value
///   is never touched unless it genuinely changes: leaving the key out, and
///   resending exactly the value `get_metadata` returned, are treated the
///   same way (nothing is checked, converted or rewritten), so an app that
///   resends every field on save is safe even when the stored value is not
///   one MeedyaManager would accept if typed fresh (COMPAT-030).  (This
///   comment used to tell apps NOT to resend an unchanged value; that
///   advice predates the fix that made resending safe, and was corrected
///   after the third review round of the language-policy work.)  A
///   `TagEntry`'s
///   `note` is ignored here — it is a report about the file, not data.
/// * **A `"language"` value is ONE value.**  One holding a zero character
///   (several values, the way a tag separates them) or any other control
///   character is refused with `MmFfiError::Metadata`, before anything is
///   written — it used to be cut at the first value silently (Codex's
///   catch-up review, finding 2).
/// * **A key is given once.**  Two entries with the same key in one write
///   are refused with `MmFfiError::Metadata`, naming the key and each value,
///   before anything is written — the last one used to win silently (the
///   stand-in review of round 6, carry-over 2).
/// * **The result says when a language write loses detail.**  On success
///   this returns a `WriteMetadataResult` whose `notes` carries, for the
///   `language` entry, the same note `meedya edit --set` shows — for
///   example that an MP3 stores `por` for `pt-BR`, losing the region — or
///   `None` when there is nothing to say (Codex's catch-up review, finding
///   6).  It used to return nothing at all.
#[uniffi::export]
pub fn write_metadata(
    path: String,
    tags: Vec<TagEntry>,
) -> Result<WriteMetadataResult, MmFfiError> {
    let file_path = PathBuf::from(&path);

    // The stand-in review of round 6, carry-over 2: the same key twice in
    // one write is refused, before anything is written, as `meedya edit`
    // refuses a field given twice. The list became a map below, so the last
    // entry silently won: reproduced with the library built from `49cec29`,
    // `[{"key":"title","value":"A"},{"key":"title","value":"B"}]` answered
    // `{"ok":true}` and the MP3 stored "B"; two `language` entries ("en",
    // then "pt-BR") answered with a note for "pt-BR" only. Which value the
    // caller meant is not something this can know.
    refuse_a_key_given_twice(&tags)?;

    // Convert Vec<TagEntry> → TagMap (HashMap<String, Vec<String>>)
    // Split "; "-delimited values back into separate entries
    let tag_map: TagMap = tags
        .into_iter()
        .map(|e| {
            // Split on "; " to reconstruct multi-value vectors
            let values = e
                .value
                .split("; ")
                .filter(|s| !s.is_empty())
                .map(std::borrow::ToOwned::to_owned)
                .collect::<Vec<_>>();
            (
                e.key,
                if values.is_empty() {
                    vec![e.value]
                } else {
                    values
                },
            )
        })
        .collect();

    // Finding 6: the note is worked out BEFORE the save, exactly as
    // `meedya edit` does — afterwards the file already holds the new value,
    // and the comparison would see "nothing changed".
    let language_note = language_write_note(&file_path, &tag_map);

    // Route through the integrity guard rather than the raw metadata layer:
    // the guard is the only enforcement point for Test Mode (issue #128), so
    // a direct call would overwrite the user's original file even with Test
    // Mode on.  The guard also gives us the atomic-rename + hash-verify
    // behaviour the native UIs would otherwise have to reimplement.
    let result = integrity::write_tags_safe(&file_path, &tag_map);

    if result.success {
        Ok(WriteMetadataResult {
            notes: language_note.map(|entry| vec![entry]),
        })
    } else {
        Err(MmFfiError::Metadata(
            result
                .error
                .unwrap_or_else(|| "metadata write failed".to_string()),
        ))
    }
}

/// `Err` naming every key `tags` holds more than once, and each value given
/// for it, in order — each value in double quotes (see [`quoted`]), and the
/// key with any invisible character shown as `\u{..}`, so the message can
/// never be cut short at a zero character or hide what differs. `Ok` when
/// every key appears once.
fn refuse_a_key_given_twice(tags: &[TagEntry]) -> Result<(), MmFfiError> {
    let mut given_by_key: Vec<(&str, Vec<&str>)> = Vec::new();
    for entry in tags {
        match given_by_key.iter_mut().find(|(k, _)| *k == entry.key) {
            Some((_, values)) => values.push(&entry.value),
            None => given_by_key.push((&entry.key, vec![&entry.value])),
        }
    }
    let twice: Vec<String> = given_by_key
        .iter()
        .filter(|(_, values)| values.len() > 1)
        .map(|(key, values)| {
            let shown: Vec<String> = values.iter().map(|v| quoted(v)).collect();
            format!(
                "'{}' is given more than once in this write ({})",
                metadata::language::show_invisible_characters(key),
                shown.join(", then ")
            )
        })
        .collect();
    if twice.is_empty() {
        return Ok(());
    }
    Err(MmFfiError::Metadata(format!(
        "{} — give each field once, so it is clear which value to save. Nothing was written.",
        twice.join("; ")
    )))
}

/// `value` in double quotes, for a message: a backslash or a double quote
/// inside it is escaped (`\\`, `\"`) and every invisible character is
/// written out as `\u{..}`.
///
/// Why the escaping (the stand-in review of round 7, N1): two entries whose
/// values were `A", then "B` and `C` gave exactly the message three entries
/// `A`, `B` and `C` give — `("A", then "B", then "C")` — reproduced through
/// the C API of the library built from `e4db8f8`. With the quote escaped,
/// the first reads `("A\", then \"B", then "C")`. Backslashes are escaped
/// first, so a value that itself holds the text `\u{200b}` shows as
/// `\\u{200b}`, never as the zero-width space written out.
fn quoted(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        "\"{}\"",
        metadata::language::show_invisible_characters(&escaped)
    )
}

/// The note to report for the `language` entry of a write, if any — the
/// same facts `meedya edit --set language=...` shows, from the same function
/// (`metadata::language::conversion_note`), worded as what happened rather
/// than as a preview, and read from the same file
/// (`integrity::where_a_save_starts`: the Test Mode copy an earlier edit
/// made, when there is one, since that is what the save changes). `None`
/// when `language` is not being written, is being cleared, or there is
/// nothing to say.
fn language_write_note(path: &std::path::Path, tag_map: &TagMap) -> Option<TagEntry> {
    let value = metadata::join_multi_value(tag_map.get(metadata::TAG_LANGUAGE)?);
    if value.is_empty() {
        return None;
    }
    let save_starts_from = integrity::where_a_save_starts(path);
    // Worked out now, before the save; worded as what happened, since the
    // caller reads it after (the stand-in review of round 6, N4).
    let note = metadata::language::conversion_note(
        &save_starts_from,
        &value,
        metadata::language::NoteTense::AfterTheSave,
    )?;
    Some(TagEntry {
        key: metadata::TAG_LANGUAGE.to_string(),
        value,
        note: Some(note),
    })
}

/// Remove a single tag field from a media file.
///
/// Uses the canonical lowercase key (e.g. "title", "artist", "album") — see
/// `mm_core::metadata::known_tag_keys` for the full list.  Removing a key the
/// file does not carry succeeds; passing a key that is not in the mapping at
/// all now returns `MmFfiError::Metadata` rather than silently succeeding
/// (issue #206) — see the note on `write_metadata`.
///
/// Honours Test Mode: with it enabled the original file is left untouched and
/// the removal is applied to the `_MeedyaManager` copy.
#[uniffi::export]
pub fn remove_tag(path: String, tag_key: String) -> Result<(), MmFfiError> {
    let file_path = PathBuf::from(&path);

    // Integrity guard, not the raw metadata layer — see `write_metadata`.
    let result = integrity::remove_tag_safe(&file_path, &tag_key);

    if result.success {
        Ok(())
    } else {
        Err(MmFfiError::Metadata(
            result
                .error
                .unwrap_or_else(|| "tag removal failed".to_string()),
        ))
    }
}

/// Read audio technical properties from a media file.
///
/// Returns duration, bitrate, sample rate, channels, and bit depth.
/// Fields that cannot be determined are set to 0.
#[uniffi::export]
pub fn get_audio_properties(path: String) -> Result<AudioPropertiesFfi, MmFfiError> {
    let file_path = PathBuf::from(&path);
    let props = metadata::extract_audio_properties(&file_path).map_err(MmFfiError::from)?;

    // Determine codec and lossless flag from the file extension via classify
    let (codec, is_lossless) = file_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| {
            let classification = classify::classify_by_extension(ext);
            let codec_str = classification.format.extension().to_ascii_uppercase();
            // Lossless formats: bit depth is typically present (Some), lossy formats return None
            let lossless = props.bits_per_sample.is_some();
            (codec_str, lossless)
        })
        .unwrap_or_else(|| ("Unknown".to_string(), false));

    Ok(AudioPropertiesFfi {
        // Convert fractional seconds to whole seconds (u32)
        duration_secs: props.duration_secs as u32,
        // Bitrate in kbps; 0 if unknown
        bitrate_kbps: props.bitrate_kbps.unwrap_or(0),
        // Sample rate in Hz; 0 if unknown
        sample_rate_hz: props.sample_rate_hz.unwrap_or(0),
        // Channel count; 0 if unknown
        channels: props.channels.unwrap_or(0),
        // Bit depth; 0 for lossy formats
        bit_depth: props.bits_per_sample.unwrap_or(0),
        is_lossless,
        codec,
    })
}

// ---------------------------------------------------------------------------
// Rule / template engine
// ---------------------------------------------------------------------------

/// Validate a rename template string.
///
/// Safe to call on every keystroke from the rule builder UI.
/// Returns a `ValidationResult` with `is_valid`, error message, and warnings.
#[uniffi::export]
pub fn validate_template(template: String) -> ValidationResult {
    // Empty / whitespace-only templates are immediately invalid
    if template.trim().is_empty() {
        return ValidationResult {
            is_valid: false,
            error_message: "Template must not be empty".into(),
            warnings: vec![],
        };
    }

    // Parse the template through the rule engine lexer + parser
    // A successful parse means the syntax is valid
    match rule_engine::parse_template(&template) {
        Ok(_ast) => ValidationResult {
            is_valid: true,
            error_message: String::new(),
            // Future: add warnings for unknown tag names here
            warnings: vec![],
        },
        Err(e) => ValidationResult {
            is_valid: false,
            error_message: e.to_string(),
            warnings: vec![],
        },
    }
}

/// Apply a rename template to a set of tags and return the computed filename.
///
/// Used by the rule builder live-preview to show the template result
/// against a sample file's metadata without touching any files.
#[uniffi::export]
pub fn apply_template(template: String, tags: Vec<TagEntry>) -> Result<String, MmFfiError> {
    // Convert Vec<TagEntry> → TagMap for EvalContext
    let tag_map: TagMap = tags.into_iter().map(|e| (e.key, vec![e.value])).collect();

    // Build an evaluation context from the tag map
    let ctx = EvalContext::new(&tag_map);

    // Parse + evaluate the template
    evaluate_template(&template, &ctx).map_err(MmFfiError::from)
}

/// List all tag display names that MeedyaManager recognises.
///
/// Returns names as used in templates (e.g. "Artist", "Title", "Album"),
/// sorted alphabetically.  Sourced dynamically from the TagRegistry so any
/// user-defined custom tags added to `tags.json5` are included automatically.
///
/// Used to populate the tag picker in the rule builder UI and by the FFI
/// layer so Swift / C# do not need to maintain their own lists.
#[uniffi::export]
pub fn list_known_tags() -> Vec<String> {
    // Delegate to the TagRegistry which loads from config/tags.json5
    // (embedded at compile time, user-overridable at runtime).
    mm_core::metadata::tag_registry::all_known_template_tags()
}

// ---------------------------------------------------------------------------
// Test Mode
// ---------------------------------------------------------------------------
//
// Mirrors `mm_core::test_mode`'s five entry points for the native UIs. Until
// these were exported, `macos/MeedyaManager/Bindings/MmCore.swift` referenced
// `testModeEnabled`, `setTestMode`, `testModeFileCount`, `commitTestModeFiles`
// and `revertTestModeFiles` with nothing on the Rust side to bind to, so the
// macOS app could not compile against the real generated bindings.

/// Return whether Test Mode is currently enabled.
///
/// Fails open (returns `false`) if the manifest cannot be read — see
/// `mm_core::test_mode::is_enabled` for why that is the safer default for a
/// UI toggle: the alternative traps the user in a Test Mode they cannot see
/// how to turn off.
#[uniffi::export]
pub fn test_mode_enabled() -> bool {
    mm_core::test_mode::is_enabled()
}

/// Enable or disable Test Mode.
///
/// Disabling does not itself commit or revert already-staged files — the UI
/// is expected to call `commit_test_mode_files` or `revert_test_mode_files`
/// around this, as `mm_core::test_mode::disable` documents.
#[uniffi::export]
pub fn set_test_mode(enabled: bool) -> Result<(), MmFfiError> {
    if enabled {
        mm_core::test_mode::enable().map_err(MmFfiError::from)
    } else {
        mm_core::test_mode::disable().map_err(MmFfiError::from)
    }
}

/// Return the number of files currently staged in Test Mode.
#[uniffi::export]
pub fn test_mode_file_count() -> u32 {
    // Manifest files are files-on-disk, never anywhere near u32::MAX.
    mm_core::test_mode::tracked_file_count() as u32
}

/// Commit all staged Test Mode files: originals are deleted and their
/// `_MeedyaManager` copies take the original names.
///
/// Returns the number of files successfully committed.
#[uniffi::export]
pub fn commit_test_mode_files() -> Result<u32, MmFfiError> {
    mm_core::test_mode::commit_files()
        .map(|committed| committed as u32)
        .map_err(MmFfiError::from)
}

/// Revert all staged Test Mode files, discarding the staged copies and
/// leaving the originals untouched.
#[uniffi::export]
pub fn revert_test_mode_files() -> Result<(), MmFfiError> {
    mm_core::test_mode::revert_files().map_err(MmFfiError::from)
}

// ---------------------------------------------------------------------------
// File watcher
// ---------------------------------------------------------------------------

/// Internal handle keeping a watcher alive and its reader thread running.
struct ActiveWatcher {
    /// The notify watcher — dropping this closes the channel sender and
    /// causes the reader thread to exit its receive loop naturally.
    _watcher: notify::RecommendedWatcher,
    /// Background thread that reads WatchEvents and forwards to the callback.
    /// Joined (cleaned up) when the handle is dropped.
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for ActiveWatcher {
    fn drop(&mut self) {
        // The _watcher field drops first, closing the channel.
        // We then join the thread to ensure the callback is not called
        // after the handle is removed from WATCHERS.
        if let Some(handle) = self.thread.take() {
            // Thread will exit shortly after the channel closes;
            // best-effort join (ignore errors from panicking threads)
            let _ = handle.join();
        }
    }
}

/// Map of active watcher handles keyed by their handle ID.
static WATCHERS: std::sync::LazyLock<Mutex<HashMap<u64, ActiveWatcher>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Atomic counter for generating unique watcher handle IDs.
static NEXT_HANDLE_ID: AtomicU64 = AtomicU64::new(1);

/// Start watching a directory for file system events.
///
/// Events are delivered to `callback` from a background thread.
/// Returns a handle ID to pass to `stop_watch` when done.
#[uniffi::export]
pub fn start_watch(directory: String, callback: Arc<dyn WatchCallback>) -> Result<u64, MmFfiError> {
    let dir_path = PathBuf::from(&directory);

    // Build a WatcherConfig for the target directory
    let config = WatcherConfig {
        folders: vec![dir_path],
        recursive: true,
        ..WatcherConfig::default()
    };

    // Start the channel-based watcher from mm-core
    let (watcher, receiver) = watcher::start_watcher(&config).map_err(MmFfiError::from)?;

    // Assign a unique ID for this watcher instance
    let handle_id = NEXT_HANDLE_ID.fetch_add(1, Ordering::SeqCst);

    // Spawn a background thread that reads WatchEvents from the channel
    // and forwards them to the UniFFI callback implementation
    let thread = std::thread::spawn(move || {
        // Block until an event arrives or the channel is closed (watcher dropped)
        while let Ok(event) = receiver.recv() {
            // Convert mm-core WatchEvent to FFI-safe WatchEventFfi
            let ffi_event = watch_event_to_ffi(event);
            // Deliver to the callback implementation (Swift / Kotlin / test)
            callback.on_event(ffi_event);
        }
        // Channel closed — watcher was stopped; thread exits cleanly
    });

    // Store the handle so stop_watch can find and drop it
    WATCHERS.lock().unwrap().insert(
        handle_id,
        ActiveWatcher {
            _watcher: watcher,
            thread: Some(thread),
        },
    );

    Ok(handle_id)
}

/// Stop a previously started directory watcher.
///
/// Removing the handle drops the watcher, which closes the event channel
/// and causes the reader thread to exit. This is a no-op for unknown IDs.
#[uniffi::export]
pub fn stop_watch(handle_id: u64) {
    // Removing from the map drops ActiveWatcher, which drops _watcher
    // (closing the channel) and joins the thread via the Drop impl.
    WATCHERS.lock().unwrap().remove(&handle_id);
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Collect all media (Audio + Video) file paths from a directory.
///
/// Uses the classify module to determine if each file is a recognised
/// media format. Other file types (documents, archives, etc.) are skipped.
pub(crate) fn collect_media_files(dir: &PathBuf, recursive: bool) -> std::io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    collect_media_files_inner(dir, recursive, &mut paths)?;
    Ok(paths)
}

/// Recursive inner helper for `collect_media_files`.
fn collect_media_files_inner(
    dir: &PathBuf,
    recursive: bool,
    out: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    use mm_core::classify::MediaGroup;

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() && recursive {
            // Recurse into sub-directory
            collect_media_files_inner(&path, recursive, out)?;
        } else if path.is_file() {
            // Skip Test Mode copies (`_MeedyaManager` suffixed duplicates).
            // A scan that picked these up would offer to rename the staged
            // copy alongside — or instead of — the real original, which is
            // exactly the confusion Test Mode exists to prevent: the copy is
            // an internal implementation detail, not a file the user asked
            // MeedyaManager to organise.
            if mm_core::test_mode::is_test_mode_copy(&path) {
                continue;
            }

            // Check the file extension against the classify module
            let is_media = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|ext| {
                    let c = classify::classify_by_extension(ext);
                    // Include Audio and Video files only; skip Image/Document/Archive
                    matches!(c.group, MediaGroup::Audio | MediaGroup::Video)
                });

            if is_media {
                out.push(path);
            }
        }
    }

    Ok(())
}

/// Flatten a `TagMap` (HashMap<String, Vec<String>>) to a flat
/// `HashMap<String, String>` by joining multi-values with "; ".
///
/// This is required because `renamer::simulate_rename` works with the
/// flat map while `metadata::extract_tags` returns the multi-value form.
fn flatten_tag_map(tag_map: TagMap) -> HashMap<String, String> {
    tag_map
        .into_iter()
        .map(|(key, values)| (key, values.join("; ")))
        .collect()
}

/// Convert a mm-core `WatchEvent` to the FFI-safe `WatchEventFfi`.
fn watch_event_to_ffi(event: WatchEvent) -> WatchEventFfi {
    match event {
        WatchEvent::Created(path) => WatchEventFfi {
            kind: "created".into(),
            path: path.to_string_lossy().into_owned(),
            new_path: String::new(),
        },
        WatchEvent::Modified(path) => WatchEventFfi {
            kind: "modified".into(),
            path: path.to_string_lossy().into_owned(),
            new_path: String::new(),
        },
        WatchEvent::Deleted(path) => WatchEventFfi {
            kind: "deleted".into(),
            path: path.to_string_lossy().into_owned(),
            new_path: String::new(),
        },
        WatchEvent::Renamed(from, to) => WatchEventFfi {
            kind: "renamed".into(),
            path: from.to_string_lossy().into_owned(),
            new_path: to.to_string_lossy().into_owned(),
        },
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
// `std::env::set_var`/`remove_var` are `unsafe` in Edition 2024 because they
// race with concurrent readers.  The test below serialises its mutation behind
// ENV_LOCK and restores the variable from `Drop`, which is the discipline the
// marker asks for.  (The crate already has a blanket `#![allow(unsafe_code)]`
// for the FFI scaffolding; this attribute documents the intent locally.)
#[allow(unsafe_code)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Process-wide lock for `MM_CONFIG_DIR` within the mm-ffi test binary.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// RAII guard that points `MM_CONFIG_DIR` at a private directory for the
    /// lifetime of one test and cleans up on drop — including on an assertion
    /// panic, which a trailing `remove_var` would skip.
    ///
    /// Built by hand rather than with `tempfile` because mm-ffi has no
    /// dev-dependency on it and adding one would touch `Cargo.lock`.
    struct ConfigDirGuard {
        dir: PathBuf,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl ConfigDirGuard {
        fn new(tag: &str) -> Self {
            let lock = ENV_LOCK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            // Nanosecond clock + a tag keeps concurrent runs from colliding.
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir().join(format!("mm_ffi_{tag}_{nanos}"));
            std::fs::create_dir_all(&dir).unwrap();

            unsafe {
                std::env::set_var("MM_CONFIG_DIR", &dir);
            }
            Self { dir, _lock: lock }
        }

        fn path(&self) -> &Path {
            &self.dir
        }
    }

    impl Drop for ConfigDirGuard {
        fn drop(&mut self) {
            unsafe {
                std::env::remove_var("MM_CONFIG_DIR");
            }
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    /// Build a minimal but *real* WAV file on disk.
    ///
    /// lofty refuses a bare 44-byte header with no `data` payload, so the
    /// fixture carries 0.1 s of 8 kHz 16-bit mono silence (1,644 bytes total).
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

    /// Copy one of mm-core's committed test media files into `dir` (see
    /// `crates/mm-core/tests/fixtures/README.md`) and return the copy.
    fn copy_core_fixture(name: &str, dir: &Path) -> PathBuf {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../mm-core/tests/fixtures")
            .join(name);
        let copy = dir.join(name);
        std::fs::copy(&source, &copy)
            .unwrap_or_else(|e| panic!("cannot copy test file {}: {e}", source.display()));
        copy
    }

    /// Third review round of the language-policy work, item 3: a WAV whose
    /// RIFF INFO chunk says "fre" and whose ID3 tag says "ger" came back
    /// from `get_metadata` as plain "fre" — the apps had no way to know the
    /// German existed. The `language` entry now carries a note the app can
    /// show; no other entry does; and the C API's JSON carries it too.
    #[test]
    fn get_metadata_reports_tags_that_disagree_about_the_language() {
        let guard = ConfigDirGuard::new("langnote");
        let path = copy_core_fixture("lang_riff_fre_id3_ger.wav", guard.path());
        // Fourth review round, item M6 (the reviewer's P10): the file had
        // no tag but the language, so "only the language entry carries a
        // note" was checked against an empty list and proved nothing — the
        // reviewer put a note on EVERY entry and this test still passed.
        // Give it a title first (the language is not sent, so it is left
        // exactly as it was — COMPAT-030).
        let mut title = metadata::TagMap::new();
        title.insert(metadata::TAG_TITLE.to_string(), vec!["Old".to_string()]);
        metadata::write_tags(&path, &title).expect("a title can be added");

        let entries = get_metadata(path.display().to_string()).expect("the file is readable");
        assert!(
            entries.iter().any(|e| e.key == metadata::TAG_TITLE),
            "the check below needs an entry other than the language: {entries:?}"
        );
        let language = entries
            .iter()
            .find(|e| e.key == "language")
            .expect("a language entry");
        assert_eq!(language.value, "fre");
        let note = language
            .note
            .as_deref()
            .expect("the ID3 tag's \"ger\" must not be hidden from the apps");
        assert!(note.contains("ID3 tag says \"ger\""), "{note}");
        assert!(
            entries
                .iter()
                .filter(|e| e.key != "language")
                .all(|e| e.note.is_none()),
            "only the language entry carries a note: {entries:?}"
        );

        // The Windows app reads the C API's JSON. The note must be in it.
        let json = serde_json::to_value(&entries).unwrap();
        let language_json = json
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["key"] == "language")
            .unwrap();
        assert_eq!(
            language_json["note"],
            serde_json::Value::String(note.to_string())
        );
    }

    /// The other side: tags that agree give no note, and the JSON keeps the
    /// exact shape it always had (no `note` key at all), so nothing that
    /// already reads it sees a change. JSON without a `note` — what the
    /// Windows app sends to `write_metadata` — must still be accepted.
    #[test]
    fn get_metadata_has_no_note_when_the_tags_agree_and_old_json_still_parses() {
        let guard = ConfigDirGuard::new("langnonote");
        let path = copy_core_fixture("riff_language.wav", guard.path());

        let entries = get_metadata(path.display().to_string()).expect("the file is readable");
        assert!(entries.iter().all(|e| e.note.is_none()), "{entries:?}");
        let json = serde_json::to_string(&entries).unwrap();
        assert!(!json.contains("\"note\""), "{json}");

        let parsed: Vec<TagEntry> =
            serde_json::from_str(r#"[{"key":"title","value":"X"}]"#).unwrap();
        assert_eq!(parsed[0].note, None);
    }

    /// The FFI write path must obey Test Mode exactly as the CLI does — the
    /// native UIs call straight into it, so an unguarded write here would let
    /// macOS/Windows clobber originals the user asked us not to touch.
    #[test]
    fn write_metadata_respects_test_mode() {
        let guard = ConfigDirGuard::new("testmode");

        let original = guard.path().join("track.wav");
        write_wav_fixture(&original);
        let before = std::fs::read(&original).unwrap();

        mm_core::test_mode::enable().expect("test mode must enable under the isolated config dir");

        write_metadata(
            original.display().to_string(),
            vec![TagEntry {
                key: "title".to_string(),
                value: "Diverted".to_string(),
                note: None,
            }],
        )
        .expect("write_metadata should succeed in Test Mode");

        assert_eq!(
            std::fs::read(&original).unwrap(),
            before,
            "Test Mode must leave the original byte-for-byte untouched"
        );

        let copy = guard.path().join("track_MeedyaManager.wav");
        assert!(copy.exists(), "Test Mode copy {} missing", copy.display());

        let tags = mm_core::metadata::extract_tags(&copy).unwrap();
        assert_eq!(
            tags.get("title").map(Vec::as_slice),
            Some(&["Diverted".to_string()][..]),
            "the copy must carry the new title"
        );
    }

    /// An unmapped key now reaches the caller as an error instead of a silent
    /// success — the behaviour change that matters most to the native UIs.
    #[test]
    fn write_metadata_rejects_unknown_key() {
        let guard = ConfigDirGuard::new("unknownkey");

        let p = guard.path().join("track.wav");
        write_wav_fixture(&p);
        let before = std::fs::read(&p).unwrap();

        let err = write_metadata(
            p.display().to_string(),
            vec![TagEntry {
                key: "bogus_key".to_string(),
                value: "1".to_string(),
                note: None,
            }],
        )
        .expect_err("an unmapped key must not report success");

        assert!(
            matches!(err, MmFfiError::Metadata(ref m) if m.contains("bogus_key")),
            "expected a Metadata error naming the key, got: {err:?}"
        );
        assert_eq!(
            std::fs::read(&p).unwrap(),
            before,
            "a rejected write must not touch the file"
        );
    }
    /// Policy MWBM-MEDIA-LANG 1.0.0: a "language" value nothing recognises is
    /// refused the same way an unknown key already is — before anything is
    /// written, as an error the native UIs can show, never silently accepted.
    #[test]
    fn write_metadata_rejects_gibberish_language() {
        let guard = ConfigDirGuard::new("gibberishlanguage");

        let p = guard.path().join("track.wav");
        write_wav_fixture(&p);
        let before = std::fs::read(&p).unwrap();

        let err = write_metadata(
            p.display().to_string(),
            vec![TagEntry {
                key: "language".to_string(),
                value: "not a language".to_string(),
                note: None,
            }],
        )
        .expect_err("a language nothing recognises must not report success");

        assert!(
            matches!(err, MmFfiError::Metadata(ref m) if m.contains("not a language")),
            "expected a Metadata error naming the rejected value, got: {err:?}"
        );
        assert_eq!(
            std::fs::read(&p).unwrap(),
            before,
            "a rejected write must not touch the file"
        );
    }

    /// Codex's catch-up review, finding 2, through the UniFFI API (the
    /// macOS app's way in): a language value holding a zero character is
    /// several values, and used to be cut at the first — a FLAC whose
    /// language was German was saved as `en`, French discarded, and the call
    /// succeeded. It must be refused, naming the problem, with the file
    /// untouched; so must any other control character.
    #[test]
    fn write_metadata_refuses_several_languages_or_a_control_character() {
        let guard = ConfigDirGuard::new("severallanguages");
        for (value, says) in [
            ("en\u{0}fr", "more than one value"),
            ("en\u{1b}fr", "control character"),
        ] {
            let path = copy_core_fixture("silence.flac", guard.path());
            let before = std::fs::read(&path).unwrap();

            let err = write_metadata(
                path.display().to_string(),
                vec![TagEntry {
                    key: "language".to_string(),
                    value: value.to_string(),
                    note: None,
                }],
            )
            .expect_err("several values or a control character must not be saved");

            assert!(
                matches!(err, MmFfiError::Metadata(ref m) if m.contains(says) && !m.contains('\u{0}')),
                "{value:?}: {err:?}"
            );
            assert_eq!(
                std::fs::read(&path).unwrap(),
                before,
                "{value:?}: file untouched"
            );
            std::fs::remove_file(&path).unwrap();
        }
    }

    /// The same, through the C API (the Windows app's way in), with Codex's
    /// own input: the JSON escape `\u0000` becomes a real zero character once
    /// the JSON is read. Reproduced with the library built from `a150926`:
    /// the answer was `{"ok":true}` and the FLAC stored `en`.
    #[test]
    fn c_api_refuses_several_languages_in_one_value_and_leaves_the_file() {
        use std::ffi::{CStr, CString};

        let guard = ConfigDirGuard::new("capiseverallanguages");
        let path = copy_core_fixture("silence.flac", guard.path());
        let before = std::fs::read(&path).unwrap();

        let c_path = CString::new(path.display().to_string()).unwrap();
        let c_json = CString::new(r#"[{"key":"language","value":"en\u0000fr"}]"#).unwrap();
        // SAFETY: both arguments are valid, zero-terminated C strings that
        // outlive the call, and the answer is freed exactly once below with
        // the library's own `mm_ffi_free_string`.
        let answer = unsafe {
            let ptr = crate::capi::mm_ffi_write_metadata(c_path.as_ptr(), c_json.as_ptr());
            let text = CStr::from_ptr(ptr).to_str().unwrap().to_string();
            crate::capi::mm_ffi_free_string(ptr.cast_mut());
            text
        };

        let json: serde_json::Value = serde_json::from_str(&answer).unwrap();
        let error = json["error"]
            .as_str()
            .unwrap_or_else(|| panic!("refused: {answer}"));
        assert!(error.contains("more than one value"), "{answer}");
        assert!(json.get("ok").is_none(), "{answer}");
        assert_eq!(
            std::fs::read(&path).unwrap(),
            before,
            "the file must be untouched"
        );
    }

    /// Codex's catch-up review, finding 6, through the UniFFI API:
    /// `pt-BR` written to an MP3 is stored as `por` — the region is lost —
    /// and the result must say so with the same note `meedya edit --set`
    /// shows. Reproduced with the library built from `a150926`: the C API
    /// answered only `{"ok":true}`, and the UniFFI call returned nothing. No
    /// note when nothing is lost (`en` on an MP3 is stored as `eng`, the
    /// same language; `pt-BR` on a FLAC is kept whole), and none for a
    /// write that does not set the language.
    #[test]
    fn write_metadata_reports_a_language_write_that_loses_detail() {
        let guard = ConfigDirGuard::new("writenotes");
        let write = |fixture: &str, key: &str, value: &str| {
            let path = copy_core_fixture(fixture, guard.path());
            let result = write_metadata(
                path.display().to_string(),
                vec![TagEntry {
                    key: key.to_string(),
                    value: value.to_string(),
                    note: None,
                }],
            )
            .unwrap_or_else(|e| panic!("{fixture} {key}={value}: {e:?}"));
            (path, result)
        };

        let (path, result) = write("silence.mp3", "language", "pt-BR");
        let notes = result
            .notes
            .expect("pt-BR on an MP3 loses the region: a note is due");
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert_eq!(
            (notes[0].key.as_str(), notes[0].value.as_str()),
            ("language", "pt-BR")
        );
        let note = notes[0].note.as_deref().unwrap_or_default();
        // The same facts the CLI shows, from the same function, read from a
        // fresh copy of the same file before any save — worded as what
        // happened, where the CLI's preview says what will happen (the
        // stand-in review of round 6, N4).
        let fresh_dir = guard.path().join("fresh");
        std::fs::create_dir_all(&fresh_dir).unwrap();
        let fresh = copy_core_fixture("silence.mp3", &fresh_dir);
        let after = metadata::language::conversion_note(
            &fresh,
            "pt-BR",
            metadata::language::NoteTense::AfterTheSave,
        );
        assert_eq!(Some(note), after.as_deref(), "the same facts as the CLI");
        assert!(
            note.contains("lost the region you typed — it was stored there as \"por\""),
            "said as what happened: {note}"
        );
        let cli_note =
            metadata::language::preview_conversion_note(&fresh, "pt-BR").expect("the CLI's note");
        assert!(
            cli_note.contains("will lose the region you typed"),
            "the CLI's preview: {cli_note}"
        );
        assert_eq!(
            metadata::extract_tags(&path).unwrap().get("language"),
            Some(&vec!["por".to_string()]),
            "and it really stored por"
        );

        for (fixture, key, value) in [
            ("silence.mp3", "language", "en"),
            ("silence.flac", "language", "pt-BR"),
            ("silence.mp3", "title", "A title"),
        ] {
            let (_, result) = write(fixture, key, value);
            assert_eq!(
                result.notes, None,
                "{fixture} {key}={value}: nothing was lost"
            );
        }
    }

    /// Finding 6 through the C API: the same note, as `"notes"` beside
    /// `"ok":true`, and the answer stays exactly `{"ok":true}` when there is
    /// nothing to say — the Windows app checks only for an `"error"` key, so
    /// both shapes keep working for it.
    #[test]
    fn c_api_write_answer_carries_the_note_only_when_there_is_one() {
        use std::ffi::{CStr, CString};

        let guard = ConfigDirGuard::new("capiwritenotes");
        let answer_for = |fixture: &str, json: &str| -> String {
            let path = copy_core_fixture(fixture, guard.path());
            let c_path = CString::new(path.display().to_string()).unwrap();
            let c_json = CString::new(json).unwrap();
            // SAFETY: both arguments are valid, zero-terminated C strings that
            // outlive the call, and the answer is freed exactly once with the
            // library's own `mm_ffi_free_string`.
            unsafe {
                let ptr = crate::capi::mm_ffi_write_metadata(c_path.as_ptr(), c_json.as_ptr());
                let text = CStr::from_ptr(ptr).to_str().unwrap().to_string();
                crate::capi::mm_ffi_free_string(ptr.cast_mut());
                text
            }
        };

        let answer = answer_for("silence.mp3", r#"[{"key":"language","value":"pt-BR"}]"#);
        let json: serde_json::Value = serde_json::from_str(&answer).unwrap();
        assert_eq!(json["ok"], serde_json::Value::Bool(true), "{answer}");
        let notes = json["notes"]
            .as_array()
            .unwrap_or_else(|| panic!("notes: {answer}"));
        assert_eq!(notes.len(), 1, "{answer}");
        assert_eq!(notes[0]["key"], "language");
        assert_eq!(notes[0]["value"], "pt-BR");
        assert!(
            notes[0]["note"]
                .as_str()
                .is_some_and(|n| n.contains("\"por\"")),
            "{answer}"
        );

        assert_eq!(
            answer_for("silence.flac", r#"[{"key":"language","value":"pt-BR"}]"#),
            r#"{"ok":true}"#,
            "nothing lost: the answer keeps its old shape exactly"
        );
    }

    /// The stand-in review of round 6, N7: a refused write's message says
    /// "Metadata error:" once. Reproduced with the library built from
    /// `49cec29`: the C API answered "Metadata error: Could not save the
    /// changes to '…': Metadata error: cannot set 'language': …".
    #[test]
    fn c_api_refusal_says_metadata_error_once() {
        use std::ffi::{CStr, CString};

        let guard = ConfigDirGuard::new("capilabelonce");
        let path = copy_core_fixture("silence.mp3", guard.path());
        let c_path = CString::new(path.display().to_string()).unwrap();
        let c_json = CString::new(r#"[{"key":"language","value":"not a language"}]"#).unwrap();
        // SAFETY: both arguments are valid, zero-terminated C strings that
        // outlive the call, and the answer is freed exactly once below with
        // the library's own `mm_ffi_free_string`.
        let answer = unsafe {
            let ptr = crate::capi::mm_ffi_write_metadata(c_path.as_ptr(), c_json.as_ptr());
            let text = CStr::from_ptr(ptr).to_str().unwrap().to_string();
            crate::capi::mm_ffi_free_string(ptr.cast_mut());
            text
        };
        let json: serde_json::Value = serde_json::from_str(&answer).unwrap();
        let error = json["error"]
            .as_str()
            .unwrap_or_else(|| panic!("must be refused: {answer}"));
        assert_eq!(
            error.matches("Metadata error:").count(),
            1,
            "the label once: {error}"
        );
        assert!(
            error.contains("'not a language' is not a language"),
            "{error}"
        );
    }

    /// The stand-in review of round 6, carry-over 2, through the UniFFI API:
    /// two entries with the same key in one write are refused, naming the
    /// key and each value, and nothing is written — whether the values
    /// differ or not, and for `language` too. Reproduced with the library
    /// built from `49cec29`: the last entry was written and the call
    /// succeeded.
    #[test]
    fn write_metadata_refuses_a_key_given_twice() {
        let guard = ConfigDirGuard::new("keytwice");
        for (fixture, key, first, second) in [
            ("silence.mp3", "title", "A", "B"),
            ("silence.flac", "title", "Same", "Same"),
            ("silence.mp3", "language", "en", "pt-BR"),
        ] {
            let path = copy_core_fixture(fixture, guard.path());
            let before = std::fs::read(&path).unwrap();
            let entry = |value: &str| TagEntry {
                key: key.to_string(),
                value: value.to_string(),
                note: None,
            };
            let err = write_metadata(
                path.display().to_string(),
                vec![entry(first), entry(second)],
            )
            .expect_err("a key given twice must be refused");
            let MmFfiError::Metadata(message) = err else {
                panic!("{key}: expected a Metadata error, got {err:?}");
            };
            assert!(
                message.contains(&format!(
                    "'{key}' is given more than once in this write (\"{first}\", then \"{second}\")"
                )),
                "{message}"
            );
            assert_eq!(
                std::fs::read(&path).unwrap(),
                before,
                "{key}: nothing written"
            );
            std::fs::remove_file(&path).unwrap();
        }
    }

    /// The stand-in review of round 7, N1 and N2: each value given for a
    /// repeated key is named (three here, not "both"), in double quotes
    /// with any quote or backslash inside it escaped — so two entries
    /// `A", then "B` and `C` can no longer read exactly like three entries
    /// `A`, `B` and `C`, as they did with the library built from `e4db8f8`.
    #[test]
    fn a_key_given_twice_names_each_value_with_quotes_escaped() {
        let guard = ConfigDirGuard::new("keytwicequotes");
        let path = copy_core_fixture("silence.mp3", guard.path());
        let refusal = |values: &[&str]| -> String {
            let entries = values
                .iter()
                .map(|value| TagEntry {
                    key: "title".to_string(),
                    value: (*value).to_string(),
                    note: None,
                })
                .collect();
            match write_metadata(path.display().to_string(), entries) {
                Err(MmFfiError::Metadata(message)) => message,
                other => panic!("{values:?}: expected a refusal, got {other:?}"),
            }
        };
        let three = refusal(&["A", "B", "C"]);
        assert!(three.contains(r#"("A", then "B", then "C")"#), "{three}");
        let two = refusal(&[r#"A", then "B"#, "C"]);
        assert!(two.contains(r#"("A\", then \"B", then "C")"#), "{two}");
        assert_ne!(two, three, "two entries must not read like three");
        let slash = refusal(&[r"back\slash", r"\u{200b}"]);
        assert!(
            slash.contains(r#"("back\\slash", then "\\u{200b}")"#),
            "a backslash is escaped, so typed text never looks written out: {slash}"
        );
    }

    /// The same through the C API, the Windows app's way in.
    #[test]
    fn c_api_refuses_a_key_given_twice() {
        use std::ffi::{CStr, CString};

        let guard = ConfigDirGuard::new("capikeytwice");
        let path = copy_core_fixture("silence.mp3", guard.path());
        let before = std::fs::read(&path).unwrap();
        let c_path = CString::new(path.display().to_string()).unwrap();
        let c_json =
            CString::new(r#"[{"key":"title","value":"A"},{"key":"title","value":"B"}]"#).unwrap();
        // SAFETY: both arguments are valid, zero-terminated C strings that
        // outlive the call, and the answer is freed exactly once below with
        // the library's own `mm_ffi_free_string`.
        let answer = unsafe {
            let ptr = crate::capi::mm_ffi_write_metadata(c_path.as_ptr(), c_json.as_ptr());
            let text = CStr::from_ptr(ptr).to_str().unwrap().to_string();
            crate::capi::mm_ffi_free_string(ptr.cast_mut());
            text
        };
        let json: serde_json::Value = serde_json::from_str(&answer).unwrap();
        let error = json["error"]
            .as_str()
            .unwrap_or_else(|| panic!("must be refused: {answer}"));
        assert!(
            error.contains("'title' is given more than once in this write (\"A\", then \"B\")"),
            "{error}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), before, "nothing written");
    }

    /// The stand-in review of round 7, M1, through the C API (the Windows
    /// app's way in). Reproduced with the library built from `e4db8f8` on a
    /// WAV whose RIFF INFO list holds the title "Café" (all UTF-8):
    /// `[{"key":"title","value":"New"}]` answered `{"ok":true}` and left
    /// "Café" in the list beside the new ID3 title (ffprobe showed both);
    /// `[{"key":"title","value":""}]` answered `{"ok":true}` and changed
    /// nothing. The list now holds exactly what was asked, so the file
    /// reads back as one title, or none.
    #[test]
    fn c_api_setting_or_clearing_a_title_a_wav_list_holds_changes_it_there_too() {
        use std::ffi::{CStr, CString};

        let guard = ConfigDirGuard::new("capiriffset");
        for (value, expected) in [("New", Some(vec!["New".to_string()])), ("", None)] {
            let path = copy_core_fixture("lang_riff_fre_all_utf8.wav", guard.path());
            let c_path = CString::new(path.display().to_string()).unwrap();
            let c_json = CString::new(format!(r#"[{{"key":"title","value":"{value}"}}]"#)).unwrap();
            // SAFETY: both arguments are valid, zero-terminated C strings
            // that outlive the call, and the answer is freed exactly once
            // below with the library's own `mm_ffi_free_string`.
            let answer = unsafe {
                let ptr = crate::capi::mm_ffi_write_metadata(c_path.as_ptr(), c_json.as_ptr());
                let text = CStr::from_ptr(ptr).to_str().unwrap().to_string();
                crate::capi::mm_ffi_free_string(ptr.cast_mut());
                text
            };
            assert_eq!(answer, r#"{"ok":true}"#, "title={value:?}");
            let tags = metadata::extract_tags(&path).unwrap();
            assert_eq!(
                tags.get("title"),
                expected.as_ref(),
                "title={value:?}: the title the file holds afterwards"
            );
            std::fs::remove_file(&path).unwrap();
        }
    }

    /// The stand-in review of round 7, L7, through the C API: an unknown key
    /// is quoted with its invisible characters written out. The reviewer
    /// read this path but did not run it; run with the library built from
    /// `e4db8f8`, a key holding a right-to-left override came back raw.
    #[test]
    fn c_api_unknown_key_is_quoted_with_invisible_characters_shown() {
        use std::ffi::{CStr, CString};

        let guard = ConfigDirGuard::new("capiunknownkey");
        let path = copy_core_fixture("silence.flac", guard.path());
        let before = std::fs::read(&path).unwrap();
        let c_path = CString::new(path.display().to_string()).unwrap();
        let c_json = CString::new("[{\"key\":\"ti\u{202e}tle\",\"value\":\"X\"}]").unwrap();
        // SAFETY: both arguments are valid, zero-terminated C strings that
        // outlive the call, and the answer is freed exactly once below with
        // the library's own `mm_ffi_free_string`.
        let answer = unsafe {
            let ptr = crate::capi::mm_ffi_write_metadata(c_path.as_ptr(), c_json.as_ptr());
            let text = CStr::from_ptr(ptr).to_str().unwrap().to_string();
            crate::capi::mm_ffi_free_string(ptr.cast_mut());
            text
        };
        let json: serde_json::Value = serde_json::from_str(&answer).unwrap();
        let error = json["error"]
            .as_str()
            .unwrap_or_else(|| panic!("must be refused: {answer}"));
        assert!(error.contains("'ti\\u{202e}tle'"), "{error}");
        assert!(!error.contains('\u{202e}'), "nothing raw: {error}");
        assert_eq!(std::fs::read(&path).unwrap(), before, "nothing written");
    }

    /// The stand-in review of round 6 (L3, its planted fault F6b): with Test
    /// Mode on and a copy an earlier edit made, the write result's note
    /// must describe the COPY — the file the save changes — not the
    /// original. The reviewer made `language_write_note` read the original
    /// and every mm-ffi test still passed. Here the original holds "en-JJ"
    /// (a region code nobody has registered), so on the original resending
    /// "en-JJ" is no change and there is nothing to say; after a first Test
    /// Mode write sets "en", the copy holds "en", so "en-JJ" is a real
    /// change there and the result must say "JJ" is not on the official
    /// list. (The same case the command line's
    /// `in_test_mode_a_note_the_copy_needs_is_not_lost` covers.)
    #[test]
    fn write_metadata_note_in_test_mode_describes_the_copy() {
        let guard = ConfigDirGuard::new("testmodenote");
        let path = copy_core_fixture("riff_language.wav", guard.path());
        let mut en_jj = TagMap::new();
        en_jj.insert("language".to_string(), vec!["en-JJ".to_string()]);
        metadata::write_tags(&path, &en_jj).expect("setup: en-JJ on the original");

        let write = |value: &str| {
            write_metadata(
                path.display().to_string(),
                vec![TagEntry {
                    key: "language".to_string(),
                    value: value.to_string(),
                    note: None,
                }],
            )
        };
        mm_core::test_mode::enable().expect("test mode must enable under the isolated config dir");
        let first = write("en");
        let second = write("en-JJ");
        mm_core::test_mode::disable().expect("test mode must disable");

        first.expect("the first write makes the copy");
        let notes = second
            .expect("en-JJ is a real language")
            .notes
            .expect("a note is due: the copy held en");
        let note = notes[0].note.as_deref().unwrap_or_default();
        assert!(
            note.contains("\"JJ\" is not on the official list"),
            "{note}"
        );
        assert_eq!(
            metadata::extract_tags(&mm_core::test_mode::test_mode_path(&path))
                .unwrap()
                .get("language"),
            Some(&vec!["en-JJ".to_string()]),
            "and the copy holds en-JJ"
        );
    }

    /// The other half of the same rule: a value LANG-002 DOES recognise —
    /// here, an old three-letter code — must be accepted and converted, not
    /// merely tolerated.
    #[test]
    fn write_metadata_accepts_a_legacy_three_letter_language() {
        let guard = ConfigDirGuard::new("legacylanguage");

        let p = guard.path().join("track.wav");
        write_wav_fixture(&p);

        write_metadata(
            p.display().to_string(),
            vec![TagEntry {
                key: "language".to_string(),
                value: "fre".to_string(),
                note: None,
            }],
        )
        .expect("a recognised legacy language code must be accepted");
    }

    /// Review item 2 of the second language-policy review round: proves
    /// COMPAT-030's "resend, don't omit" case through the FFI boundary the
    /// native UIs actually call — `write_tags_rejects_gibberish_language`
    /// (mm-core) and `resending_an_unchanged_language_value_is_never_
    /// refused` (the metadata_roundtrip integration test) prove the harder
    /// cases (a value nothing could parse fresh) one layer down; mm-ffi has
    /// no direct dependency on `lofty` to poke a raw value in the way those
    /// two tests do, so this proves the same property for the ordinary
    /// case every native UI actually hits on every save: resending a value
    /// this crate itself wrote a moment ago.
    #[test]
    fn write_metadata_resends_an_unchanged_language_without_refusal() {
        let guard = ConfigDirGuard::new("resendlanguage");

        let p = guard.path().join("track.wav");
        write_wav_fixture(&p);

        write_metadata(
            p.display().to_string(),
            vec![TagEntry {
                key: "language".to_string(),
                value: "fre".to_string(),
                note: None,
            }],
        )
        .expect("setting a recognised legacy language code must succeed");

        // "fre" is French's BIBLIOGRAPHIC ISO 639-2 form; TRACK-070 always
        // writes the TERMINOLOGY form into an ID3 tag (this WAV gets an
        // embedded one — see `wav_write_tags_uses_embedded_id3v2_not_riff_info`
        // in `metadata_roundtrip.rs`), so what actually lands on disk is
        // "fra", not "fre" — read back rather than assumed, since resending
        // the WRONG text would make this test look like it proves the
        // COMPAT-030 property without actually exercising it.
        let stored = mm_core::metadata::extract_tags(&p)
            .unwrap_or_else(|e| panic!("extract_tags failed: {e}"))
            .get("language")
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            stored,
            vec!["fra".to_string()],
            "sanity check: this is what must actually be resent below"
        );

        // RESENDING the exact value that is already there — key present,
        // not omitted, which is what every native UI in this project
        // actually does on every save (see `write_tags`'s own doc
        // comment) — must not be refused.
        write_metadata(
            p.display().to_string(),
            vec![TagEntry {
                key: "language".to_string(),
                value: stored[0].clone(),
                note: None,
            }],
        )
        .expect("resending the unchanged value \"fra\" must not be refused");
    }

    /// Review item 7 of the second language-policy review round, MUST FIX:
    /// `integrity::mutate_file_safe` used to build its failure message as
    /// `"mutation failed on '{target}': {e}"`, where `target` is Test
    /// Mode's own internal `_MeedyaManager` copy path — leaking that
    /// plumbing straight into a message the native apps show verbatim.
    /// Enables Test Mode (unlike `write_metadata_rejects_gibberish_language`
    /// above, which does not, so it never exercised the diverted-target
    /// code path this bug lived in) and checks the refusal names the
    /// rejected VALUE, never a copy.
    ///
    /// Corrected after the third review round (item 8, a decision of the
    /// lead's): this used to require the message to name NO path at all.
    /// The fault was naming the INTERNAL copy; the lead decided every
    /// failure should instead say "Could not save the changes to '<the
    /// file you chose>'", so the person's own file is now named on purpose
    /// and this checks for exactly that.
    #[test]
    fn write_metadata_refusal_names_the_real_file_never_a_copy() {
        let guard = ConfigDirGuard::new("refusalnopath");

        let p = guard.path().join("track.wav");
        write_wav_fixture(&p);
        mm_core::test_mode::enable().expect("test mode must enable under the isolated config dir");

        let err = write_metadata(
            p.display().to_string(),
            vec![TagEntry {
                key: "language".to_string(),
                value: "not a language".to_string(),
                note: None,
            }],
        )
        .expect_err("a language nothing recognises must not report success");

        let message = match err {
            MmFfiError::Metadata(m) => m,
            other => panic!("expected MmFfiError::Metadata, got: {other:?}"),
        };
        assert!(
            message.contains("not a language"),
            "must still name the rejected value: {message:?}"
        );
        assert!(
            !message.contains("_MeedyaManager"),
            "must not name Test Mode's internal copy: {message:?}"
        );
        assert!(
            message.starts_with(&format!(
                "Could not save the changes to '{}': ",
                p.display()
            )),
            "must start with plain words and the file the app asked to change: {message:?}"
        );
        assert!(
            !message.contains("meedya_tmp") && !message.contains("mutation failed"),
            "must not name the scratch copy or use internal jargon: {message:?}"
        );
    }

    // ── The write lock — issue #49 ──────────────────────────────────────────

    /// **Regression — two copies moving files at once.**
    ///
    /// The desktop apps call `execute_renames` straight through this
    /// function. Before the fix it took no lock at all, so pressing the
    /// rename button while a `meedya scan --execute` was running in a
    /// terminal had both processes moving the same files.
    ///
    /// Rewritten for the #49 lock redesign: the previous version of this
    /// test planted a *file naming this process's own PID* to stand in for a
    /// second holder. That plant no longer means anything under the new
    /// design — the lock is the operating system's own file lock, not
    /// anything read out of the file's content — so this now genuinely
    /// takes the lock first, via the same `LockFile` API a real second
    /// process would use, and keeps it alive for the whole test.
    #[test]
    fn execute_renames_refuses_while_the_lock_is_held() {
        let guard = ConfigDirGuard::new("writelock");

        let source = guard.path().join("before.wav");
        write_wav_fixture(&source);
        let original_bytes = std::fs::read(&source).unwrap();
        let destination = guard.path().join("after.wav");

        // Genuinely hold the write lock, standing in for a second live copy
        // of MeedyaManager already moving files.
        let lock_path = mm_core::state::LockFile::default_path();
        let held_lock = mm_core::state::LockFile::try_acquire(&lock_path, "another test")
            .expect("acquiring must not error")
            .expect("nothing else holds this lock yet");

        let result = execute_renames(vec![RenamePreviewFfi {
            source: source.display().to_string(),
            destination: destination.display().to_string(),
            conflict: false,
            unchanged: false,
        }]);

        assert!(
            result.is_err(),
            "execute_renames must refuse to move anything while another \
             process holds the write lock: {result:?}"
        );
        assert!(
            source.is_file(),
            "the source file must still be where it started"
        );
        assert_eq!(
            std::fs::read(&source).unwrap(),
            original_bytes,
            "the source file must be byte-for-byte unchanged"
        );
        assert!(
            !destination.exists(),
            "nothing may have been written to the destination"
        );

        drop(held_lock);
    }
}
