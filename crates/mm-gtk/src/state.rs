// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — GTK Application State
//
// All non-GTK application state lives here so it can be constructed and
// tested without display access.  GTK widgets hold references to these
// structs (wrapped in Rc<RefCell<...>>) to share state between panels.

use std::collections::HashMap;
use std::path::PathBuf;

use mm_core::renamer::RenamePreview;

// ---------------------------------------------------------------------------
// Scan panel state
// ---------------------------------------------------------------------------

/// State for the Library / Scan panel.
#[derive(Debug, Default)]
pub struct ScanState {
    /// The directory to scan (None if not yet selected)
    pub directory: Option<PathBuf>,
    /// The rename template string (e.g. "<Artist> - <Title>")
    pub template: String,
    /// Whether to scan sub-directories recursively
    pub recursive: bool,
    /// Rename previews from the last scan (empty before first scan)
    pub previews: Vec<RenamePreview>,
    /// Human-readable status message for the status bar
    pub status: String,
}

impl ScanState {
    /// Create a new scan state with sensible defaults.
    pub fn new() -> Self {
        Self {
            directory: None,
            template: "<Artist> - <Title>".to_string(),
            recursive: false,
            previews: vec![],
            status: "Select a folder to begin scanning.".to_string(),
        }
    }

    /// Return a summary string describing the current previews.
    pub fn preview_summary(&self) -> String {
        if self.previews.is_empty() {
            return "No files scanned yet.".to_string();
        }

        let total = self.previews.len();
        let renamed = self.previews.iter().filter(|p| !p.unchanged && !p.conflict).count();
        let unchanged = self.previews.iter().filter(|p| p.unchanged).count();
        let conflicts = self.previews.iter().filter(|p| p.conflict).count();

        format!(
            "{total} files — {renamed} to rename, {unchanged} unchanged, {conflicts} conflicts"
        )
    }

    /// Return only the previews that are ready to execute (no conflict, not unchanged).
    pub fn executable_previews(&self) -> Vec<&RenamePreview> {
        self.previews
            .iter()
            .filter(|p| !p.unchanged && !p.conflict)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Metadata panel state
// ---------------------------------------------------------------------------

/// State for the Metadata editor panel.
#[derive(Debug, Default)]
pub struct MetadataState {
    /// The file currently being edited (None if not yet selected)
    pub file_path: Option<PathBuf>,
    /// All tags read from the file (key → joined value)
    pub tags: HashMap<String, String>,
    /// Edits made by the user (key → new value) — not yet saved
    pub pending_edits: HashMap<String, String>,
    /// Human-readable status message
    pub status: String,
}

impl MetadataState {
    /// Create a new metadata state.
    pub fn new() -> Self {
        Self {
            file_path: None,
            tags: HashMap::new(),
            pending_edits: HashMap::new(),
            status: "Select a media file to view its metadata.".to_string(),
        }
    }

    /// Return true if there are unsaved edits.
    pub fn has_pending_edits(&self) -> bool {
        !self.pending_edits.is_empty()
    }

    /// Merge pending edits back into the main tag map (after a save).
    pub fn commit_edits(&mut self) {
        for (key, value) in self.pending_edits.drain() {
            self.tags.insert(key, value);
        }
    }

    /// Discard all pending edits without saving.
    pub fn discard_edits(&mut self) {
        self.pending_edits.clear();
    }
}

// ---------------------------------------------------------------------------
// Unit tests — no GTK required
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- ScanState tests ---

    /// New ScanState has the expected defaults
    #[test]
    fn scan_state_defaults() {
        let state = ScanState::new();
        assert!(state.directory.is_none());
        assert_eq!(state.template, "<Artist> - <Title>");
        assert!(!state.recursive);
        assert!(state.previews.is_empty());
        assert!(!state.status.is_empty());
    }

    /// preview_summary returns "No files scanned yet." when empty
    #[test]
    fn scan_state_summary_empty() {
        let state = ScanState::new();
        assert_eq!(state.preview_summary(), "No files scanned yet.");
    }

    /// preview_summary correctly counts renamed / unchanged / conflict
    #[test]
    fn scan_state_summary_with_previews() {
        use std::path::PathBuf;
        let mut state = ScanState::new();
        state.previews = vec![
            // To rename
            RenamePreview {
                source: PathBuf::from("/a.mp3"),
                destination: PathBuf::from("/Pink Floyd - Shine On.mp3"),
                conflict: false,
                unchanged: false,
            },
            // Unchanged
            RenamePreview {
                source: PathBuf::from("/b.mp3"),
                destination: PathBuf::from("/b.mp3"),
                conflict: false,
                unchanged: true,
            },
            // Conflict
            RenamePreview {
                source: PathBuf::from("/c.mp3"),
                destination: PathBuf::from("/exists.mp3"),
                conflict: true,
                unchanged: false,
            },
        ];

        let summary = state.preview_summary();
        assert!(summary.contains("3 files"));
        assert!(summary.contains("1 to rename"));
        assert!(summary.contains("1 unchanged"));
        assert!(summary.contains("1 conflicts"));
    }

    /// executable_previews excludes unchanged and conflicted entries
    #[test]
    fn scan_state_executable_previews() {
        use std::path::PathBuf;
        let mut state = ScanState::new();
        state.previews = vec![
            RenamePreview { source: PathBuf::from("/a.mp3"), destination: PathBuf::from("/new_a.mp3"), conflict: false, unchanged: false },
            RenamePreview { source: PathBuf::from("/b.mp3"), destination: PathBuf::from("/b.mp3"),     conflict: false, unchanged: true },
            RenamePreview { source: PathBuf::from("/c.mp3"), destination: PathBuf::from("/conflict.mp3"), conflict: true, unchanged: false },
        ];

        let exec = state.executable_previews();
        assert_eq!(exec.len(), 1);
        assert_eq!(exec[0].source, PathBuf::from("/a.mp3"));
    }

    // --- MetadataState tests ---

    /// New MetadataState has the expected defaults
    #[test]
    fn metadata_state_defaults() {
        let state = MetadataState::new();
        assert!(state.file_path.is_none());
        assert!(state.tags.is_empty());
        assert!(!state.has_pending_edits());
    }

    /// has_pending_edits returns true after adding an edit
    #[test]
    fn metadata_state_pending_edits() {
        let mut state = MetadataState::new();
        assert!(!state.has_pending_edits());

        state.pending_edits.insert("title".into(), "New Title".into());
        assert!(state.has_pending_edits());
    }

    /// commit_edits merges pending edits into the tag map
    #[test]
    fn metadata_state_commit_edits() {
        let mut state = MetadataState::new();
        state.tags.insert("artist".into(), "Old Artist".into());
        state.pending_edits.insert("artist".into(), "New Artist".into());
        state.pending_edits.insert("title".into(), "Track Title".into());

        state.commit_edits();

        assert_eq!(state.tags["artist"], "New Artist");
        assert_eq!(state.tags["title"], "Track Title");
        assert!(!state.has_pending_edits());
    }

    /// discard_edits clears pending edits without touching saved tags
    #[test]
    fn metadata_state_discard_edits() {
        let mut state = MetadataState::new();
        state.tags.insert("artist".into(), "Original".into());
        state.pending_edits.insert("artist".into(), "Discarded".into());

        state.discard_edits();

        assert!(!state.has_pending_edits());
        assert_eq!(state.tags["artist"], "Original"); // unchanged
    }

    // --- Template tests using mm-core rule engine ---

    /// The default template is parseable by the rule engine
    #[test]
    fn default_template_is_valid() {
        let state = ScanState::new();
        // Parse the default template through the rule engine to verify it's valid
        let result = mm_core::rule_engine::parse_template(&state.template);
        assert!(result.is_ok(), "Default template should parse without error: {:?}", result.err());
    }
}
