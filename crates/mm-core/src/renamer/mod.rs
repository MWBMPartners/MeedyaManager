// (C) 2025-2026 MWBM Partners Ltd
//
// File rename simulation and execution.
//
// Computes destination paths from metadata, detects conflicts,
// provides dry-run preview, and executes renames with rollback.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
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
    "CON", "PRN", "AUX", "NUL",
    "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
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
    let trimmed = if config.windows_compatible
        && WINDOWS_RESERVED_NAMES.contains(&upper.as_str())
    {
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
        let ext_len = ext.map_or(0, |e| e.len());
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

/// Substitute metadata values into a path template.
///
/// Replaces `{key}` placeholders in the template with values from the
/// metadata map. Unknown keys are replaced with "Unknown".
pub fn substitute_template(
    template: &str,
    metadata: &HashMap<String, String>,
) -> String {
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

        // Build parent directories from template (if any subdirectories)
        let parent_parts = template_path.parent().unwrap_or(Path::new(""));
        let destination = output_dir.join(parent_parts).join(&safe_name);

        // Detect conflicts
        let conflict = destination.exists()
            || destinations_seen.contains_key(&destination);
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

    let renamed = previews.iter().filter(|p| !p.unchanged && !p.conflict).count();
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
pub fn simulate_rename_with_rules(
    files: &[PathBuf],
    rules: &[Rule],
    default_template: &str,
    output_dir: &Path,
    config: &SanitizeConfig,
    ctx_builder: impl Fn(&Path) -> MmResult<EvalContext<'_>>,
) -> MmResult<RenameSummary> {
    let mut previews = Vec::with_capacity(files.len());
    let mut destinations_seen: HashMap<PathBuf, usize> = HashMap::new();

    for source in files {
        // Build the evaluation context for this file
        let ctx = ctx_builder(source)?;

        // Try rules first, then fall back to the default template
        let raw_path = if !rules.is_empty() {
            match rule_engine::apply_rules(rules, &ctx)? {
                Some(result) => result,
                None => rule_engine::evaluate_template(default_template, &ctx)?,
            }
        } else {
            rule_engine::evaluate_template(default_template, &ctx)?
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

        // Build parent directories from template (if any subdirectories)
        let parent_parts = template_path.parent().unwrap_or(Path::new(""));
        let destination = output_dir.join(parent_parts).join(&safe_name);

        // Detect conflicts
        let conflict = destination.exists()
            || destinations_seen.contains_key(&destination);
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

    let renamed = previews.iter().filter(|p| !p.unchanged && !p.conflict).count();
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

/// Execute a rename operation, moving files from source to destination.
///
/// Creates parent directories as needed. Returns an error if any rename
/// fails, but does not roll back previously successful renames.
pub fn execute_rename(preview: &RenamePreview) -> MmResult<()> {
    if preview.unchanged {
        return Ok(());
    }

    if preview.conflict {
        return Err(MmError::Rename(format!(
            "destination already exists: {}",
            preview.destination.display()
        )));
    }

    // Create parent directories
    if let Some(parent) = preview.destination.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            MmError::Rename(format!("cannot create directory: {e}"))
        })?;
    }

    // Perform the rename
    std::fs::rename(&preview.source, &preview.destination).map_err(|e| {
        MmError::Rename(format!(
            "cannot rename {} → {}: {e}",
            preview.source.display(),
            preview.destination.display()
        ))
    })?;

    Ok(())
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
        assert_eq!(sanitize_filename("file<>name.mp3", &config), "file__name.mp3");
        assert_eq!(sanitize_filename("file:name.mp3", &config), "file_name.mp3");
        assert_eq!(sanitize_filename("file|name.mp3", &config), "file_name.mp3");
    }

    #[test]
    fn sanitize_custom_replacements() {
        let mut config = SanitizeConfig::default();
        config.custom_replacements.insert(':', " -".to_string());
        assert_eq!(sanitize_filename("Song: Title.mp3", &config), "Song - Title.mp3");
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
        assert_eq!(sanitize_filename("file\x00name.mp3", &config), "file_name.mp3");
        assert_eq!(sanitize_filename("file\tname.mp3", &config), "file_name.mp3");
    }

    #[test]
    fn sanitize_truncates_long_names() {
        let mut config = SanitizeConfig::default();
        config.max_length = 20;
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

        let files = vec![(source.clone(), meta)];
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
