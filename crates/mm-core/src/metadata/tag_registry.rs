// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Metadata Tag Registry
//
// Provides a lazy-initialised, process-global registry of all metadata tag
// definitions that MeedyaManager recognises.  Data is sourced from
// `config/tags.json5` (embedded at compile time) with an optional user
// override at:
//
//   Linux/macOS:  ~/.config/meedyamanager/tags.json5
//   Windows:      %APPDATA%\MeedyaManager\tags.json5
//
// Public API:
//   - tag_definitions()         — all standard tag definitions (slice)
//   - custom_tag_definitions()  — user-defined MeedyaMeta custom tags (slice)
//   - tag_by_id(id)             — lookup by internal key (e.g. "title")
//   - tag_by_name(name)         — lookup by template name (e.g. "Title")
//   - known_template_tags()     — sorted list of template names (for template
//                                 validation and UI pickers)
//   - known_template_tags_for_category(cat) — filtered by category
//
// ## Relationship to upstream `meedya_core::metadata::tag_registry`
//
// MeedyaManager's local registry is a **UI / template** registry: it maps
// template display names like `<Artist>` to internal MM string IDs plus
// per-format frame hints (id3, vorbis, mp4, ape) and a category for the
// rename builder UI.  It is populated from `config/tags.json5`.
//
// The upstream registry (`meedya_core::metadata::TagRegistry`) is a
// **provider / API-JSON** registry: it maps tag IDs to JSON-extraction
// paths plus target atom namespaces, and is populated from a TOML file
// shipped by the consuming MeedyaSuite tool.  It serves the
// MeedyaDL-style "API response → atom write" flow.
//
// These two registries solve different problems and are not interchangeable.
// We keep the local UI registry as-is, and additionally re-export the
// upstream provider-registry types under `provider::*` for future use by
// MeedyaManager's metadata-provider integration (M5+).

use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Upstream provider-registry re-exports
// ---------------------------------------------------------------------------

/// Re-exports of the upstream config-driven provider tag registry.
///
/// Use these types when wiring metadata providers that emit raw JSON
/// responses needing to be mapped to file-level atoms via a TOML config
/// (the MeedyaDL flow).  They are distinct from the local UI-facing
/// [`TagDefinition`] / [`TagCategory`] / [`CustomTagDefinition`] types
/// declared below, which drive the rename rule engine and tag pickers.
pub mod provider {
    pub use meedya_core::metadata::tag_registry::{
        AtomTarget, TagDefinition, TagRegistry, TagScope, TagValueType,
    };

    /// Upstream JSON-path extraction helper — used by `write_registry_tags`
    /// and other provider-side functions to dig into raw API responses.
    pub use meedya_core::metadata::json_path::{extract_json_value, value_to_string};
}

// ---------------------------------------------------------------------------
// Embedded default registry — compiled into every binary build
// ---------------------------------------------------------------------------

/// Built-in `tags.json5`, embedded at compile time.
static DEFAULT_JSON5: &str = include_str!("../../../../config/tags.json5");

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// The category a tag belongs to.
///
/// Categories are used by the UI to group tags into logical sections and to
/// filter which tags appear in which context (e.g. only `virtual` tags appear
/// in the "file properties" section of the rename builder).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TagCategory {
    /// Universally supported core tags (title, artist, album, year, …)
    Core,
    /// Sort-key variants of core tags (for Apple Music / foobar2000 ordering)
    Sort,
    /// Extended attribution (conductor, remixer, lyricist, mood, …)
    Extended,
    /// Classical music-specific fields (work, movement, …)
    Classical,
    /// ReplayGain loudness-normalisation values
    Replaygain,
    /// Encoding process information (encoder name, settings, …)
    Encoding,
    /// MusicBrainz globally-unique identifiers
    Musicbrainz,
    /// Podcast-specific fields
    Podcast,
    /// Computed/virtual tags — not stored in file metadata, available in
    /// rename templates only (Filename, Extension, Duration, …)
    Virtual,
    /// User-defined custom tags in the MeedyaMeta namespace
    Custom,
}

/// Definition of a standard metadata tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagDefinition {
    /// Internal key used throughout MeedyaManager (lowercase_snake_case).
    /// Corresponds to the `TAG_*` constants in `metadata/mod.rs`.
    pub id: String,

    /// Template display name (PascalCase, e.g. `"Artist"`).
    /// Used inside rename templates: `<Artist>`.
    pub name: String,

    /// Short human-readable description shown in the UI.
    pub desc: String,

    /// Logical category for grouping in the UI.
    pub category: TagCategory,

    /// Whether this tag commonly holds multiple values (e.g. multiple artists).
    #[serde(default)]
    pub multi: bool,

    /// ID3v2 frame identifier (e.g. `"TPE1"`, `"TXXX:ARTIST"`).
    #[serde(default)]
    pub id3: Option<String>,

    /// Vorbis comment key (e.g. `"ARTIST"`).
    #[serde(default)]
    pub vorbis: Option<String>,

    /// MP4 / iTunes atom name (e.g. `"©ART"`).
    #[serde(default)]
    pub mp4: Option<String>,

    /// APE tag key (e.g. `"Artist"`).
    #[serde(default)]
    pub ape: Option<String>,

    /// MusicBrainz field name (for provider lookups).
    #[serde(default)]
    pub mb: Option<String>,

    /// When `false` the tag is hidden from the UI.
    /// Defaults to `true` when the field is absent from JSON5.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// A user-defined custom tag in the MeedyaMeta namespace.
///
/// Custom tags are read/written using the raw key (e.g. `MEEDYAMETA_RATING`)
/// and stored as free-form text in whichever tag format the file uses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTagDefinition {
    /// Internal key — must start with `"custom_"` or `"meedyameta_"`.
    pub id: String,

    /// Template display name for use in rename templates.
    pub name: String,

    /// Short description shown in the UI.
    pub desc: String,

    /// Raw key written directly into the file tag (e.g. `"MEEDYAMETA_RATING"`).
    /// UPPERCASE by convention.
    pub raw_key: String,

    /// When `false` the custom tag is ignored by all lookups.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// Top-level deserialization target
// ---------------------------------------------------------------------------

/// Internal wrapper that maps the top-level keys of `tags.json5`.
#[derive(Debug, Deserialize)]
struct TagRegistryData {
    /// Standard tag definitions.
    tags: Vec<TagDefinition>,
    /// User-defined MeedyaMeta custom tags.
    custom: Vec<CustomTagDefinition>,
}

// ---------------------------------------------------------------------------
// Lazy-initialised process-global registry
// ---------------------------------------------------------------------------

/// Returns `true`; used as `#[serde(default = "default_true")]`.
fn default_true() -> bool {
    true
}

/// The singleton registry, initialised once on first access.
static REGISTRY: LazyLock<TagRegistryData> = LazyLock::new(|| {
    // Try user override file first.
    if let Some(user_json5) = load_user_override() {
        match json5::from_str::<TagRegistryData>(&user_json5) {
            Ok(data) => {
                tracing::info!("tag registry: loaded from user override file");
                return data;
            }
            Err(err) => {
                tracing::warn!(
                    "tag registry: user override file is malformed ({err}) \
                     — falling back to built-in defaults"
                );
            }
        }
    }

    // Fall back to compiled-in defaults.
    json5::from_str(DEFAULT_JSON5)
        .expect("built-in config/tags.json5 is malformed — this is a compile-time defect")
});

/// Try to read the user's custom `tags.json5` from the OS config directory.
fn load_user_override() -> Option<String> {
    let config_root = dirs::config_dir()?;
    let path = config_root.join("meedyamanager").join("tags.json5");
    std::fs::read_to_string(&path).ok()
}

// ---------------------------------------------------------------------------
// Public slice accessors
// ---------------------------------------------------------------------------

/// All standard tag definitions (including disabled ones).
///
/// Suitable for rendering the full tag list in the settings UI.
pub fn tag_definitions() -> &'static [TagDefinition] {
    &REGISTRY.tags
}

/// All user-defined MeedyaMeta custom tag definitions (including disabled).
pub fn custom_tag_definitions() -> &'static [CustomTagDefinition] {
    &REGISTRY.custom
}

// ---------------------------------------------------------------------------
// Lookup helpers (only enabled entries)
// ---------------------------------------------------------------------------

/// Return the `TagDefinition` whose `id` matches (case-insensitive).
///
/// Returns `None` if no enabled standard tag has that internal key.
pub fn tag_by_id(id: &str) -> Option<&'static TagDefinition> {
    let lower = id.to_ascii_lowercase();
    REGISTRY.tags.iter().find(|t| t.enabled && t.id == lower)
}

/// Return the `TagDefinition` whose template `name` matches (case-insensitive).
///
/// Returns `None` if no enabled standard tag has that display name.
pub fn tag_by_name(name: &str) -> Option<&'static TagDefinition> {
    let lower = name.to_ascii_lowercase();
    REGISTRY
        .tags
        .iter()
        .find(|t| t.enabled && t.name.to_ascii_lowercase() == lower)
}

/// Return the `CustomTagDefinition` whose `id` matches (case-insensitive).
pub fn custom_tag_by_id(id: &str) -> Option<&'static CustomTagDefinition> {
    let lower = id.to_ascii_lowercase();
    REGISTRY.custom.iter().find(|t| t.enabled && t.id == lower)
}

// ---------------------------------------------------------------------------
// Template tag list helpers
// ---------------------------------------------------------------------------

/// Return a sorted, deduplicated list of template display names for all
/// enabled standard tags.
///
/// This is used by:
/// - The template validator (to check `<Tag>` references in rename rules)
/// - The tag picker in the rename-rule builder UI
/// - The FFI layer (`uniffi_api::list_known_tags`, `mm_ffi_list_known_tags`)
pub fn known_template_tags() -> Vec<String> {
    let mut names: Vec<String> = REGISTRY
        .tags
        .iter()
        .filter(|t| t.enabled)
        .map(|t| t.name.clone())
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}

/// Return a sorted list of template display names filtered by category.
///
/// Useful for building grouped tag pickers in the UI.
pub fn known_template_tags_for_category(category: &TagCategory) -> Vec<String> {
    let mut names: Vec<String> = REGISTRY
        .tags
        .iter()
        .filter(|t| t.enabled && &t.category == category)
        .map(|t| t.name.clone())
        .collect();
    names.sort_unstable();
    names
}

/// Return a sorted list of all custom tag template names.
pub fn known_custom_template_tags() -> Vec<String> {
    let mut names: Vec<String> = REGISTRY
        .custom
        .iter()
        .filter(|t| t.enabled)
        .map(|t| t.name.clone())
        .collect();
    names.sort_unstable();
    names
}

/// Return sorted template names for all enabled tags across standard +
/// custom registries.  Used for exhaustive template validation.
pub fn all_known_template_tags() -> Vec<String> {
    let mut names = known_template_tags();
    names.extend(known_custom_template_tags());
    names.sort_unstable();
    names.dedup();
    names
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── Registry loads without panic ────────────────────────────────────────

    #[test]
    fn default_json5_parses_successfully() {
        // Triggers LazyLock initialisation — will panic if JSON5 is malformed.
        assert!(!tag_definitions().is_empty());
    }

    // ── Non-empty ───────────────────────────────────────────────────────────

    #[test]
    fn standard_tags_non_empty() {
        assert!(!tag_definitions().is_empty(), "tag list must not be empty");
    }

    // ── Core tag lookups ────────────────────────────────────────────────────

    #[test]
    fn tag_by_id_title() {
        let t = tag_by_id("title").expect("'title' tag must be in registry");
        assert_eq!(t.name, "Title");
        assert_eq!(t.category, TagCategory::Core);
        assert!(!t.multi);
    }

    #[test]
    fn tag_by_id_artist() {
        let t = tag_by_id("artist").expect("'artist' tag must be in registry");
        assert_eq!(t.name, "Artist");
        assert!(t.multi, "artist is multi-value");
    }

    #[test]
    fn tag_by_id_is_case_insensitive() {
        assert!(tag_by_id("TITLE").is_some());
        assert!(tag_by_id("Title").is_some());
        assert!(tag_by_id("title").is_some());
    }

    // ── Lookup by display name ───────────────────────────────────────────────

    #[test]
    fn tag_by_name_title() {
        let t = tag_by_name("Title").expect("'Title' name must be in registry");
        assert_eq!(t.id, "title");
    }

    #[test]
    fn tag_by_name_is_case_insensitive() {
        assert!(tag_by_name("TITLE").is_some());
        assert!(tag_by_name("title").is_some());
    }

    #[test]
    fn tag_by_name_missing_returns_none() {
        assert!(tag_by_name("NoSuchTag12345").is_none());
    }

    // ── Known template tags list ─────────────────────────────────────────────

    #[test]
    fn known_template_tags_contains_core() {
        let tags = known_template_tags();
        assert!(tags.contains(&"Title".to_string()));
        assert!(tags.contains(&"Artist".to_string()));
        assert!(tags.contains(&"Album".to_string()));
        assert!(tags.contains(&"Year".to_string()));
    }

    #[test]
    fn known_template_tags_contains_virtual() {
        let tags = known_template_tags();
        assert!(tags.contains(&"Filename".to_string()));
        assert!(tags.contains(&"Extension".to_string()));
        assert!(tags.contains(&"Duration".to_string()));
    }

    #[test]
    fn known_template_tags_is_sorted() {
        let tags = known_template_tags();
        let mut sorted = tags.clone();
        sorted.sort_unstable();
        assert_eq!(
            tags, sorted,
            "known_template_tags() must return a sorted list"
        );
    }

    #[test]
    fn known_template_tags_no_duplicates() {
        let tags = known_template_tags();
        let mut deduped = tags.clone();
        deduped.dedup();
        assert_eq!(
            tags.len(),
            deduped.len(),
            "no duplicate names in template tag list"
        );
    }

    // ── Category filter ──────────────────────────────────────────────────────

    #[test]
    fn category_filter_core_contains_title() {
        let core = known_template_tags_for_category(&TagCategory::Core);
        assert!(core.contains(&"Title".to_string()));
        assert!(core.contains(&"Artist".to_string()));
    }

    #[test]
    fn category_filter_virtual_contains_filename() {
        let virtual_tags = known_template_tags_for_category(&TagCategory::Virtual);
        assert!(virtual_tags.contains(&"Filename".to_string()));
        assert!(virtual_tags.contains(&"Extension".to_string()));
    }

    #[test]
    fn category_filter_virtual_excludes_title() {
        let virtual_tags = known_template_tags_for_category(&TagCategory::Virtual);
        assert!(!virtual_tags.contains(&"Title".to_string()));
    }

    // ── MusicBrainz tags ─────────────────────────────────────────────────────

    #[test]
    fn musicbrainz_track_id_in_registry() {
        let t = tag_by_id("mb_track_id").expect("MusicBrainz track ID must be in registry");
        assert_eq!(t.category, TagCategory::Musicbrainz);
        assert!(t.mb.is_some());
    }

    // ── Tag uniqueness ───────────────────────────────────────────────────────

    #[test]
    fn tag_ids_are_unique() {
        let mut ids: Vec<&str> = tag_definitions().iter().map(|t| t.id.as_str()).collect();
        ids.sort_unstable();
        let original_len = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "Duplicate tag id detected");
    }

    #[test]
    fn tag_names_are_unique() {
        let mut names: Vec<&str> = tag_definitions().iter().map(|t| t.name.as_str()).collect();
        names.sort_unstable();
        let original_len = names.len();
        names.dedup();
        assert_eq!(names.len(), original_len, "Duplicate tag name detected");
    }

    // ── Custom tags (empty by default) ───────────────────────────────────────

    #[test]
    fn custom_tags_empty_by_default() {
        // The built-in tags.json5 ships with an empty custom array.
        // Users populate it in their override file.
        assert!(
            custom_tag_definitions().is_empty(),
            "built-in custom tag list should be empty — users add their own"
        );
    }

    // ── All known template tags ──────────────────────────────────────────────

    #[test]
    fn all_known_includes_standard_and_custom() {
        let all = all_known_template_tags();
        // Must contain standard core tags
        assert!(all.contains(&"Title".to_string()));
        // Must be sorted and deduped
        let mut sorted = all.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(all, sorted);
    }

    // ── Upstream provider-registry re-export smoke tests ────────────────────

    #[test]
    fn upstream_provider_registry_resolves() {
        // Build a trivial upstream registry from inline TOML to confirm
        // the re-exported types are usable from MM code without referencing
        // the meedya_metadata crate directly.
        let toml = r#"
[album.upc]
json_path = "attributes.upc"
value_type = "string"
atoms = [ { namespace = "itunes", name = "UPC" } ]

[track.isrc]
json_path = "attributes.isrc"
value_type = "string"
atoms = [ { namespace = "itunes", name = "ISRC" } ]
"#;
        let registry = provider::TagRegistry::from_toml(toml).expect("parse TOML");
        assert_eq!(registry.album_tags.len(), 1);
        assert_eq!(registry.track_tags.len(), 1);

        // Verify TagScope round-trips and find_tag works.
        let (def, scope) = registry.find_tag("isrc").expect("find isrc");
        assert_eq!(scope, provider::TagScope::Track);
        assert_eq!(def.value_type, provider::TagValueType::String);
        assert_eq!(def.atoms[0].namespace, "com.apple.iTunes");
    }

    #[test]
    fn upstream_json_path_helpers_resolve() {
        // Smoke test for the re-exported extract_json_value / value_to_string
        let j = serde_json::json!({"attributes": {"name": "Midnights"}});
        let v = provider::extract_json_value(&j, "attributes.name").expect("path resolves");
        let s = provider::value_to_string(&v, &provider::TagValueType::String)
            .expect("string conversion");
        assert_eq!(s, "Midnights");
    }
}
