// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — GTK Application State (M6)
//
// All non-GTK application state lives here so it can be constructed and
// tested without display access.  GTK widgets hold references to these
// structs (wrapped in Rc<RefCell<...>>) to share state between panels.
//
// State structs (one per tab/panel):
//   ScanState       — Library / Scan tab
//   MetadataState   — Metadata Editor tab
//   RulesState      — Rules / Template Builder tab
//   LookupState     — Metadata Lookup tab
//   SettingsSnapshot — Application Settings tab

use std::collections::HashMap;
use std::path::PathBuf;

use mm_core::renamer::RenamePreview;

// ---------------------------------------------------------------------------
// Lookup result — a single provider search hit
// ---------------------------------------------------------------------------

/// A single metadata search result returned by a provider.
#[derive(Debug, Clone, Default)]
pub struct LookupResult {
    /// Name of the provider that returned this result (e.g. "musicbrainz")
    pub provider: String,
    /// Track/show/podcast title
    pub title: Option<String>,
    /// Artist, author, or podcast host
    pub artist: Option<String>,
    /// Album or series name
    pub album: Option<String>,
    /// Release year
    pub year: Option<u32>,
    /// Genre tag
    pub genre: Option<String>,
    /// Provider-specific identifier
    pub provider_id: String,
    /// Match score [0.0, 1.0+] — higher is better
    pub score: f64,
    /// URL of the highest-resolution cover art found
    pub cover_art_url: Option<String>,
}

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
        self.previews.iter().filter(|p| !p.unchanged && !p.conflict).collect()
    }

    /// Return the number of conflicting previews.
    pub fn conflict_count(&self) -> usize {
        self.previews.iter().filter(|p| p.conflict).count()
    }

    /// Return the number of unchanged previews.
    pub fn unchanged_count(&self) -> usize {
        self.previews.iter().filter(|p| p.unchanged).count()
    }

    /// Return true if there are previews ready to execute.
    pub fn can_execute(&self) -> bool {
        !self.executable_previews().is_empty()
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
    /// All tags read from the file (key → joined value string)
    pub tags: HashMap<String, String>,
    /// Edits made by the user (key → new value) — not yet saved
    pub pending_edits: HashMap<String, String>,
    /// Human-readable status message
    pub status: String,
    /// Cover art URL extracted from the file or a lookup result (if any)
    pub cover_art_url: Option<String>,
}

impl MetadataState {
    /// Create a new metadata state.
    pub fn new() -> Self {
        Self {
            file_path: None,
            tags: HashMap::new(),
            pending_edits: HashMap::new(),
            status: "Select a media file to view its metadata.".to_string(),
            cover_art_url: None,
        }
    }

    /// Return true if there are unsaved edits.
    pub fn has_pending_edits(&self) -> bool {
        !self.pending_edits.is_empty()
    }

    /// Merge pending edits back into the main tag map (after a successful save).
    pub fn commit_edits(&mut self) {
        for (key, value) in self.pending_edits.drain() {
            self.tags.insert(key, value);
        }
    }

    /// Discard all pending edits without saving.
    pub fn discard_edits(&mut self) {
        self.pending_edits.clear();
    }

    /// Return the display value for a tag (pending edit takes precedence over saved value).
    pub fn effective_value(&self, key: &str) -> Option<&str> {
        self.pending_edits
            .get(key)
            .or_else(|| self.tags.get(key))
            .map(String::as_str)
    }

    /// Return the number of tags currently loaded.
    pub fn tag_count(&self) -> usize {
        self.tags.len()
    }
}

// ---------------------------------------------------------------------------
// Rules panel state
// ---------------------------------------------------------------------------

/// State for the Rules / Template Builder panel.
#[derive(Debug, Default)]
pub struct RulesState {
    /// The current rename template string
    pub template: String,
    /// Last validation result: None=not validated, Some(Ok(()))=valid, Some(Err(msg))=invalid
    pub validation: Option<Result<(), String>>,
    /// Live preview output (template applied to sample tags)
    pub preview_output: String,
    /// Sample tag values used to drive the live preview
    pub sample_tags: HashMap<String, String>,
}

impl RulesState {
    /// Create a new rules state with sample data pre-populated.
    pub fn new() -> Self {
        let mut sample_tags = HashMap::new();
        sample_tags.insert("Artist".into(), "Pink Floyd".into());
        sample_tags.insert("Title".into(), "Comfortably Numb".into());
        sample_tags.insert("Album".into(), "The Wall".into());
        sample_tags.insert("Year".into(), "1979".into());
        sample_tags.insert("TrackNumber".into(), "06".into());
        sample_tags.insert("Genre".into(), "Rock".into());
        sample_tags.insert("AlbumArtist".into(), "Pink Floyd".into());
        sample_tags.insert("Composer".into(), "Roger Waters".into());
        sample_tags.insert("Disc".into(), "1".into());

        let template = "<Artist> - <Title>".to_owned();
        let preview_output = Self::apply_sample(&sample_tags, &template);

        Self { template, validation: None, preview_output, sample_tags }
    }

    /// Compute the live preview by substituting sample tags into the template.
    ///
    /// Uses simple `<Tag>` string replacement (not the full rule engine evaluator)
    /// so the preview updates instantaneously on every keystroke.
    pub fn compute_preview(&self) -> String {
        Self::apply_sample(&self.sample_tags, &self.template)
    }

    fn apply_sample(sample_tags: &HashMap<String, String>, template: &str) -> String {
        let mut output = template.to_owned();
        for (key, value) in sample_tags {
            output = output.replace(&format!("<{key}>"), value);
        }
        output
    }

    /// Return true if the current template is known valid.
    pub fn is_valid(&self) -> bool {
        matches!(self.validation, Some(Ok(())))
    }

    /// Return true if the current template is known invalid.
    pub fn is_invalid(&self) -> bool {
        matches!(self.validation, Some(Err(_)))
    }

    /// Return the validation error message, if any.
    pub fn error_message(&self) -> Option<&str> {
        match &self.validation {
            Some(Err(msg)) => Some(msg.as_str()),
            _ => None,
        }
    }

    /// Update the template, recompute the preview, and clear prior validation.
    pub fn set_template(&mut self, template: impl Into<String>) {
        self.template = template.into();
        self.preview_output = self.compute_preview();
        self.validation = None;
    }
}

// ---------------------------------------------------------------------------
// Lookup panel state
// ---------------------------------------------------------------------------

/// State for the Metadata Lookup panel.
#[derive(Debug, Default)]
pub struct LookupState {
    /// The main search query string (title or keyword)
    pub query: String,
    /// Optional artist hint to refine the search
    pub artist_hint: String,
    /// Enabled state per provider name
    pub providers: HashMap<String, bool>,
    /// Results from the last search
    pub results: Vec<LookupResult>,
    /// Index of the currently selected result (None if none selected)
    pub selected: Option<usize>,
    /// Whether a search is in progress
    pub searching: bool,
    /// Status message displayed at the bottom of the panel
    pub status: String,
}

impl LookupState {
    /// Create lookup state with all known providers; concrete providers enabled.
    pub fn new() -> Self {
        let mut providers = HashMap::new();
        // Concrete providers — enabled by default
        for name in &[
            "musicbrainz", "spotify", "apple_music", "deezer",
            "tmdb", "thetvdb", "omdb", "apple_tv", "itunes_store",
            "apple_podcasts", "isrc", "eidr", "iswc",
        ] {
            providers.insert(name.to_string(), true);
        }
        // Stub providers — disabled by default (no public API)
        for name in &["youtube_music", "amazon_music", "pandora", "tidal", "shazam", "iheart"] {
            providers.insert(name.to_string(), false);
        }

        Self {
            query: String::new(),
            artist_hint: String::new(),
            providers,
            results: Vec::new(),
            selected: None,
            searching: false,
            status: "Enter a title to search for metadata.".into(),
        }
    }

    /// Return names of all enabled providers, sorted alphabetically.
    pub fn enabled_providers(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.providers
            .iter()
            // Rust 2024 forbids `&pattern` inside an implicitly-borrowing
            // closure parameter, so destructure plainly and deref twice
            // (HashMap iter yields &(&K, &V), so enabled: &&bool).
            .filter(|(_, enabled)| **enabled)
            .map(|(name, _)| name.as_str())
            .collect();
        names.sort();
        names
    }

    /// Return the currently selected result, if any.
    pub fn selected_result(&self) -> Option<&LookupResult> {
        self.selected.and_then(|i| self.results.get(i))
    }

    /// Clear results and reset selection.
    pub fn clear_results(&mut self) {
        self.results.clear();
        self.selected = None;
    }

    /// Return true if there are results to display.
    pub fn has_results(&self) -> bool {
        !self.results.is_empty()
    }

    /// Toggle a provider's enabled state.
    pub fn toggle_provider(&mut self, name: &str) {
        if let Some(enabled) = self.providers.get_mut(name) {
            *enabled = !*enabled;
        }
    }
}

// ---------------------------------------------------------------------------
// Settings panel snapshot
// ---------------------------------------------------------------------------

/// Snapshot of the editable settings shown in the Settings panel.
///
/// This is a simple `Clone`-able struct (unlike `AppConfig` which pulls in
/// many deps) that the settings panel uses to track in-flight edits.
#[derive(Debug, Clone)]
pub struct SettingsSnapshot {
    /// Dry-run mode: preview renames without moving files
    pub dry_run: bool,
    /// Watch sub-directories recursively
    pub recursive: bool,
    /// Event debounce delay in milliseconds
    pub debounce_ms: u64,
    /// Log verbosity level (trace/debug/info/warn/error)
    pub log_level: String,
    /// Redact PII (file paths and personal data) in log output
    pub redact_pii: bool,
}

impl Default for SettingsSnapshot {
    fn default() -> Self {
        Self {
            dry_run: false,
            recursive: true,
            debounce_ms: 500,
            log_level: "info".into(),
            redact_pii: false,
        }
    }
}

impl SettingsSnapshot {
    /// Return true if the log level is one of the five valid values.
    pub fn is_valid_log_level(level: &str) -> bool {
        matches!(level, "trace" | "debug" | "info" | "warn" | "error")
    }

    /// If the log level is not valid, reset it to "info".
    pub fn sanitise_log_level(&mut self) {
        if !Self::is_valid_log_level(&self.log_level) {
            self.log_level = "info".into();
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests — 35 tests, no GTK display access required
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ========================
    // ScanState (9)
    // ========================

    #[test]
    fn scan_state_defaults() {
        let s = ScanState::new();
        assert!(s.directory.is_none());
        assert_eq!(s.template, "<Artist> - <Title>");
        assert!(!s.recursive);
        assert!(s.previews.is_empty());
        assert!(!s.status.is_empty());
    }

    #[test]
    fn scan_state_summary_empty() {
        assert_eq!(ScanState::new().preview_summary(), "No files scanned yet.");
    }

    #[test]
    fn scan_state_summary_with_previews() {
        let mut s = ScanState::new();
        s.previews = vec![
            RenamePreview { source: "/a.mp3".into(), destination: "/n.mp3".into(), conflict: false, unchanged: false },
            RenamePreview { source: "/b.mp3".into(), destination: "/b.mp3".into(), conflict: false, unchanged: true  },
            RenamePreview { source: "/c.mp3".into(), destination: "/x.mp3".into(), conflict: true,  unchanged: false },
        ];
        let sum = s.preview_summary();
        assert!(sum.contains("3 files"));
        assert!(sum.contains("1 to rename"));
        assert!(sum.contains("1 unchanged"));
        assert!(sum.contains("1 conflicts"));
    }

    #[test]
    fn scan_state_executable_previews() {
        let mut s = ScanState::new();
        s.previews = vec![
            RenamePreview { source: "/a.mp3".into(), destination: "/na.mp3".into(), conflict: false, unchanged: false },
            RenamePreview { source: "/b.mp3".into(), destination: "/b.mp3".into(),  conflict: false, unchanged: true  },
            RenamePreview { source: "/c.mp3".into(), destination: "/x.mp3".into(),  conflict: true,  unchanged: false },
        ];
        assert_eq!(s.executable_previews().len(), 1);
    }

    #[test]
    fn scan_state_conflict_count() {
        let mut s = ScanState::new();
        s.previews = vec![
            RenamePreview { source: "/a.mp3".into(), destination: "/x.mp3".into(), conflict: true,  unchanged: false },
            RenamePreview { source: "/b.mp3".into(), destination: "/y.mp3".into(), conflict: true,  unchanged: false },
            RenamePreview { source: "/c.mp3".into(), destination: "/c.mp3".into(), conflict: false, unchanged: true  },
        ];
        assert_eq!(s.conflict_count(), 2);
    }

    #[test]
    fn scan_state_unchanged_count() {
        let mut s = ScanState::new();
        s.previews = vec![
            RenamePreview { source: "/a.mp3".into(), destination: "/a.mp3".into(), conflict: false, unchanged: true  },
            RenamePreview { source: "/b.mp3".into(), destination: "/b.mp3".into(), conflict: false, unchanged: true  },
            RenamePreview { source: "/c.mp3".into(), destination: "/n.mp3".into(), conflict: false, unchanged: false },
        ];
        assert_eq!(s.unchanged_count(), 2);
    }

    #[test]
    fn scan_state_can_execute_false_when_empty() {
        assert!(!ScanState::new().can_execute());
    }

    #[test]
    fn scan_state_can_execute_true_when_renameable() {
        let mut s = ScanState::new();
        s.previews = vec![
            RenamePreview { source: "/a.mp3".into(), destination: "/new.mp3".into(), conflict: false, unchanged: false },
        ];
        assert!(s.can_execute());
    }

    #[test]
    fn default_template_is_valid() {
        let result = mm_core::rule_engine::parse_template(&ScanState::new().template);
        assert!(result.is_ok(), "Default template should parse without error: {:?}", result.err());
    }

    // ========================
    // MetadataState (7)
    // ========================

    #[test]
    fn metadata_state_defaults() {
        let s = MetadataState::new();
        assert!(s.file_path.is_none());
        assert!(s.tags.is_empty());
        assert!(!s.has_pending_edits());
        assert!(s.cover_art_url.is_none());
    }

    #[test]
    fn metadata_state_pending_edits_detected() {
        let mut s = MetadataState::new();
        s.pending_edits.insert("title".into(), "New Title".into());
        assert!(s.has_pending_edits());
    }

    #[test]
    fn metadata_state_commit_edits() {
        let mut s = MetadataState::new();
        s.tags.insert("artist".into(), "Old".into());
        s.pending_edits.insert("artist".into(), "New".into());
        s.pending_edits.insert("title".into(), "Track".into());
        s.commit_edits();
        assert_eq!(s.tags["artist"], "New");
        assert_eq!(s.tags["title"], "Track");
        assert!(!s.has_pending_edits());
    }

    #[test]
    fn metadata_state_discard_edits() {
        let mut s = MetadataState::new();
        s.tags.insert("artist".into(), "Original".into());
        s.pending_edits.insert("artist".into(), "Discarded".into());
        s.discard_edits();
        assert!(!s.has_pending_edits());
        assert_eq!(s.tags["artist"], "Original");
    }

    #[test]
    fn metadata_state_effective_value_prefers_pending() {
        let mut s = MetadataState::new();
        s.tags.insert("title".into(), "Saved".into());
        s.pending_edits.insert("title".into(), "Pending".into());
        assert_eq!(s.effective_value("title"), Some("Pending"));
    }

    #[test]
    fn metadata_state_effective_value_falls_back_to_tags() {
        let mut s = MetadataState::new();
        s.tags.insert("artist".into(), "Saved Artist".into());
        assert_eq!(s.effective_value("artist"), Some("Saved Artist"));
    }

    #[test]
    fn metadata_state_tag_count() {
        let mut s = MetadataState::new();
        s.tags.insert("a".into(), "1".into());
        s.tags.insert("b".into(), "2".into());
        assert_eq!(s.tag_count(), 2);
    }

    // ========================
    // RulesState (9)
    // ========================

    #[test]
    fn rules_state_defaults() {
        let s = RulesState::new();
        assert_eq!(s.template, "<Artist> - <Title>");
        assert!(s.validation.is_none());
        assert!(!s.sample_tags.is_empty());
    }

    #[test]
    fn rules_state_sample_tags_populated() {
        let s = RulesState::new();
        assert!(s.sample_tags.contains_key("Artist"));
        assert!(s.sample_tags.contains_key("Title"));
        assert!(s.sample_tags.contains_key("Album"));
    }

    #[test]
    fn rules_state_compute_preview_substitutes_tags() {
        let s = RulesState::new();
        // Default: "<Artist> - <Title>" with Artist="Pink Floyd", Title="Comfortably Numb"
        assert_eq!(s.compute_preview(), "Pink Floyd - Comfortably Numb");
    }

    #[test]
    fn rules_state_compute_preview_unknown_tag_kept() {
        let mut s = RulesState::new();
        s.template = "<UnknownTag> - <Title>".into();
        let preview = s.compute_preview();
        assert!(preview.contains("<UnknownTag>"));
        assert!(preview.contains("Comfortably Numb"));
    }

    #[test]
    fn rules_state_is_valid_false_when_none() {
        assert!(!RulesState::new().is_valid());
    }

    #[test]
    fn rules_state_is_valid_true_after_ok() {
        let mut s = RulesState::new();
        s.validation = Some(Ok(()));
        assert!(s.is_valid());
        assert!(!s.is_invalid());
    }

    #[test]
    fn rules_state_is_invalid_true_after_err() {
        let mut s = RulesState::new();
        s.validation = Some(Err("unclosed bracket".into()));
        assert!(s.is_invalid());
        assert!(!s.is_valid());
    }

    #[test]
    fn rules_state_error_message_returned() {
        let mut s = RulesState::new();
        s.validation = Some(Err("bad template".into()));
        assert_eq!(s.error_message(), Some("bad template"));
    }

    #[test]
    fn rules_state_set_template_updates_preview() {
        let mut s = RulesState::new();
        s.set_template("<Album> - <Year>");
        assert_eq!(s.template, "<Album> - <Year>");
        assert_eq!(s.preview_output, "The Wall - 1979");
        assert!(s.validation.is_none());
    }

    // ========================
    // LookupState (8)
    // ========================

    #[test]
    fn lookup_state_defaults() {
        let s = LookupState::new();
        assert!(s.query.is_empty());
        assert!(s.results.is_empty());
        assert!(s.selected.is_none());
        assert!(!s.searching);
    }

    #[test]
    fn lookup_state_has_all_providers() {
        assert_eq!(LookupState::new().providers.len(), 19);
    }

    #[test]
    fn lookup_state_concrete_providers_enabled() {
        let s = LookupState::new();
        let enabled = s.enabled_providers();
        assert!(enabled.contains(&"musicbrainz"));
        assert!(enabled.contains(&"spotify"));
        assert!(enabled.contains(&"tmdb"));
    }

    #[test]
    fn lookup_state_stub_providers_disabled() {
        let s = LookupState::new();
        assert!(!s.providers["youtube_music"]);
        assert!(!s.providers["amazon_music"]);
        assert!(!s.providers["tidal"]);
    }

    #[test]
    fn lookup_state_toggle_provider() {
        let mut s = LookupState::new();
        assert!(s.providers["musicbrainz"]);
        s.toggle_provider("musicbrainz");
        assert!(!s.providers["musicbrainz"]);
        s.toggle_provider("musicbrainz");
        assert!(s.providers["musicbrainz"]);
    }

    #[test]
    fn lookup_state_clear_results() {
        let mut s = LookupState::new();
        s.results.push(LookupResult { title: Some("Track".into()), ..Default::default() });
        s.selected = Some(0);
        s.clear_results();
        assert!(!s.has_results());
        assert!(s.selected.is_none());
    }

    #[test]
    fn lookup_state_selected_result() {
        let mut s = LookupState::new();
        s.results.push(LookupResult { title: Some("Track 1".into()), ..Default::default() });
        s.results.push(LookupResult { title: Some("Track 2".into()), ..Default::default() });
        s.selected = Some(1);
        assert_eq!(s.selected_result().unwrap().title.as_deref(), Some("Track 2"));
    }

    #[test]
    fn lookup_state_selected_result_none_out_of_bounds() {
        let mut s = LookupState::new();
        s.selected = Some(99);
        assert!(s.selected_result().is_none());
    }

    // ========================
    // SettingsSnapshot (4)
    // ========================

    #[test]
    fn settings_snapshot_defaults() {
        let snap = SettingsSnapshot::default();
        assert!(!snap.dry_run);
        assert!(snap.recursive);
        assert_eq!(snap.log_level, "info");
        assert!(!snap.redact_pii);
        assert!(snap.debounce_ms > 0);
    }

    #[test]
    fn settings_snapshot_valid_log_levels() {
        for level in &["trace", "debug", "info", "warn", "error"] {
            assert!(SettingsSnapshot::is_valid_log_level(level));
        }
        assert!(!SettingsSnapshot::is_valid_log_level("verbose"));
        assert!(!SettingsSnapshot::is_valid_log_level(""));
    }

    #[test]
    fn settings_snapshot_sanitise_falls_back_to_info() {
        let mut snap = SettingsSnapshot::default();
        snap.log_level = "verbose".into();
        snap.sanitise_log_level();
        assert_eq!(snap.log_level, "info");
    }

    #[test]
    fn settings_snapshot_valid_level_unchanged() {
        let mut snap = SettingsSnapshot::default();
        snap.log_level = "warn".into();
        snap.sanitise_log_level();
        assert_eq!(snap.log_level, "warn");
    }

    // ========================
    // LookupResult (2)
    // ========================

    #[test]
    fn lookup_result_default_score_zero() {
        let r = LookupResult::default();
        assert_eq!(r.score, 0.0);
        assert!(r.title.is_none());
    }

    #[test]
    fn lookup_result_clone_is_independent() {
        let mut r = LookupResult { title: Some("A".into()), score: 0.9, ..Default::default() };
        let r2 = r.clone();
        r.title = Some("B".into());
        assert_eq!(r2.title.as_deref(), Some("A"));
    }
}
