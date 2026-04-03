// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Rule Engine Evaluator
//
// Walks the AST produced by the parser and evaluates it against an
// `EvalContext` to produce a final string (typically a file path).
//
// Key concepts:
//   - `EvalContext` — aggregates all data needed during evaluation:
//     tags, audio properties, classification, file path
//   - `MissingTagMode` — controls behavior when a tag is not found
//   - `evaluate()` — recursive AST walker
//   - `evaluate_template()` — convenience: parse + evaluate in one call
//
// License: GPL-2.0-or-later

use std::path::Path;

use crate::classify::MediaClassification;
use crate::error::{MmError, MmResult};
use crate::metadata::{AudioProperties, TagMap};

use super::functions;
use super::parser::{self, Node};
use super::tag_registry::{self, TagKind, VirtualTag};

// ───────────────────────────────────────────────────────────────────────────
// MissingTagMode
// ───────────────────────────────────────────────────────────────────────────

/// Controls what happens when a tag is not found during evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MissingTagMode {
    /// Return an empty string (default — silent)
    #[default]
    Empty,
    /// Return the tag name in angle brackets, e.g. "<Artist>"
    Literal,
    /// Return an error
    Error,
}

// ───────────────────────────────────────────────────────────────────────────
// EvalContext
// ───────────────────────────────────────────────────────────────────────────

/// All data needed to evaluate a template against a specific media file.
///
/// The context borrows its data to avoid cloning tag maps and properties
/// on every evaluation call.
pub struct EvalContext<'a> {
    /// Tag values extracted from the file's embedded metadata
    pub tags: &'a TagMap,
    /// Technical audio properties (duration, bitrate, sample rate, etc.)
    pub audio_props: Option<&'a AudioProperties>,
    /// 4-level media classification (group, format, class, quality)
    pub classification: Option<&'a MediaClassification>,
    /// Absolute path to the source file (for virtual tags)
    pub file_path: Option<&'a Path>,
    /// When true, multi-value tags return only the first value (for path building)
    pub path_mode: bool,
    /// What to do when a tag is missing from the context
    pub missing_tag_mode: MissingTagMode,
}

impl<'a> EvalContext<'a> {
    /// Create a new EvalContext with required tags and sensible defaults.
    pub fn new(tags: &'a TagMap) -> Self {
        Self {
            tags,
            audio_props: None,
            classification: None,
            file_path: None,
            path_mode: true, // default to path mode for file renaming
            missing_tag_mode: MissingTagMode::Empty,
        }
    }

    /// Set audio properties on this context (builder pattern).
    pub fn with_audio_props(mut self, props: &'a AudioProperties) -> Self {
        self.audio_props = Some(props);
        self
    }

    /// Set media classification on this context (builder pattern).
    pub fn with_classification(mut self, classification: &'a MediaClassification) -> Self {
        self.classification = Some(classification);
        self
    }

    /// Set the source file path on this context (builder pattern).
    pub fn with_file_path(mut self, path: &'a Path) -> Self {
        self.file_path = Some(path);
        self
    }

    /// Set path mode (builder pattern).
    pub fn with_path_mode(mut self, path_mode: bool) -> Self {
        self.path_mode = path_mode;
        self
    }

    /// Set missing tag mode (builder pattern).
    pub fn with_missing_tag_mode(mut self, mode: MissingTagMode) -> Self {
        self.missing_tag_mode = mode;
        self
    }

    // ── Tag resolution ──────────────────────────────────────────────

    /// Resolve a tag name to its string value from this context.
    ///
    /// Looks up the tag in the registry, then resolves it from the
    /// appropriate source (metadata, virtual computation, or custom tags).
    pub fn resolve_tag(&self, name: &str) -> MmResult<String> {
        // Look up in the tag registry
        match tag_registry::lookup_tag(name) {
            Some(TagKind::Metadata(key)) => self.resolve_metadata_tag(key, name),
            Some(TagKind::Virtual(vt)) => self.resolve_virtual_tag(vt, name),
            Some(TagKind::Custom(key)) => self.resolve_metadata_tag(&key, name),
            None => {
                // Try direct lookup in the tag map (allows arbitrary tags)
                let lower = name.to_lowercase();
                self.resolve_metadata_tag(&lower, name)
            }
        }
    }

    /// Resolve a metadata tag from the TagMap.
    fn resolve_metadata_tag(&self, key: &str, display_name: &str) -> MmResult<String> {
        match self.tags.get(key) {
            Some(values) if !values.is_empty() => {
                if self.path_mode {
                    // Path mode: return only the first value
                    Ok(values[0].clone())
                } else {
                    // Display mode: join all values with "; "
                    Ok(values.join("; "))
                }
            }
            _ => self.handle_missing(display_name),
        }
    }

    /// Resolve a virtual tag from context data.
    fn resolve_virtual_tag(&self, vt: VirtualTag, display_name: &str) -> MmResult<String> {
        match vt {
            VirtualTag::Filename => match self.file_path {
                Some(p) => Ok(p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string()),
                None => self.handle_missing(display_name),
            },
            VirtualTag::Extension => match self.file_path {
                Some(p) => Ok(p
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string()),
                None => self.handle_missing(display_name),
            },
            VirtualTag::Folder => match self.file_path {
                Some(p) => Ok(p
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string()),
                None => self.handle_missing(display_name),
            },
            VirtualTag::FullPath => match self.file_path {
                Some(p) => Ok(p.to_string_lossy().to_string()),
                None => self.handle_missing(display_name),
            },
            VirtualTag::Duration => match self.audio_props {
                Some(props) => {
                    // Format as M:SS or H:MM:SS
                    let total_secs = props.duration_secs as u64;
                    let hours = total_secs / 3600;
                    let mins = (total_secs % 3600) / 60;
                    let secs = total_secs % 60;
                    if hours > 0 {
                        Ok(format!("{hours}:{mins:02}:{secs:02}"))
                    } else {
                        Ok(format!("{mins}:{secs:02}"))
                    }
                }
                None => self.handle_missing(display_name),
            },
            VirtualTag::DurationSecs => match self.audio_props {
                Some(props) => Ok(format!("{}", props.duration_secs as u64)),
                None => self.handle_missing(display_name),
            },
            VirtualTag::BitrateKbps => match self.audio_props {
                Some(props) => match props.bitrate_kbps {
                    Some(br) => Ok(br.to_string()),
                    None => self.handle_missing(display_name),
                },
                None => self.handle_missing(display_name),
            },
            VirtualTag::SampleRateHz => match self.audio_props {
                Some(props) => match props.sample_rate_hz {
                    Some(sr) => Ok(sr.to_string()),
                    None => self.handle_missing(display_name),
                },
                None => self.handle_missing(display_name),
            },
            VirtualTag::Channels => match self.audio_props {
                Some(props) => match props.channels {
                    Some(ch) => Ok(ch.to_string()),
                    None => self.handle_missing(display_name),
                },
                None => self.handle_missing(display_name),
            },
            VirtualTag::BitDepth => match self.audio_props {
                Some(props) => match props.bits_per_sample {
                    Some(bd) => Ok(bd.to_string()),
                    None => self.handle_missing(display_name),
                },
                None => self.handle_missing(display_name),
            },
            VirtualTag::MediaClass => match self.classification {
                Some(c) => Ok(c.class.to_string()),
                None => self.handle_missing(display_name),
            },
            VirtualTag::MediaGroup => match self.classification {
                Some(c) => Ok(c.group.to_string()),
                None => self.handle_missing(display_name),
            },
            VirtualTag::MediaFormat => match self.classification {
                Some(c) => Ok(c.format.to_string()),
                None => self.handle_missing(display_name),
            },
            VirtualTag::MediaQuality => match self.classification {
                Some(c) => Ok(c.quality.to_string()),
                None => self.handle_missing(display_name),
            },
        }
    }

    /// Handle a missing tag according to the configured mode.
    fn handle_missing(&self, display_name: &str) -> MmResult<String> {
        match self.missing_tag_mode {
            MissingTagMode::Empty => Ok(String::new()),
            MissingTagMode::Literal => Ok(format!("<{display_name}>")),
            MissingTagMode::Error => Err(MmError::RuleEngine(format!(
                "missing tag: <{display_name}>"
            ))),
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// AST evaluation
// ───────────────────────────────────────────────────────────────────────────

/// Recursively evaluate an AST node against the given context.
///
/// Returns the evaluated string result.  Function arguments are evaluated
/// before being passed to the function implementation.
pub fn evaluate(node: &Node, ctx: &EvalContext<'_>) -> MmResult<String> {
    match node {
        // Literal text — return as-is
        Node::Literal(s) => Ok(s.clone()),

        // Tag reference — resolve from context
        Node::Tag(name) => ctx.resolve_tag(name),

        // Function call — evaluate args, then dispatch
        Node::FuncCall { name, args } => {
            // Evaluate each argument node to a string
            let evaluated_args: Vec<String> = args
                .iter()
                .map(|arg| evaluate(arg, ctx))
                .collect::<MmResult<Vec<String>>>()?;
            // Call the function implementation
            functions::eval_func(name, &evaluated_args, ctx)
        }

        // Sequence — evaluate each child, concatenate results
        Node::Sequence(nodes) => {
            let mut result = String::new();
            for child in nodes {
                result.push_str(&evaluate(child, ctx)?);
            }
            Ok(result)
        }
    }
}

/// Parse a template string and evaluate it against the given context.
///
/// This is the primary entry point for template evaluation.  It combines
/// parsing and evaluation in a single call.
///
/// # Examples
///
/// ```
/// use mm_core::rule_engine::evaluator::{EvalContext, evaluate_template};
/// use mm_core::metadata::TagMap;
///
/// let mut tags = TagMap::new();
/// tags.insert("artist".into(), vec!["Pink Floyd".into()]);
/// tags.insert("album".into(), vec!["The Wall".into()]);
///
/// let ctx = EvalContext::new(&tags);
/// let result = evaluate_template("<Artist>/<Album>", &ctx).unwrap();
/// assert_eq!(result, "Pink Floyd/The Wall");
/// ```
pub fn evaluate_template(template: &str, ctx: &EvalContext<'_>) -> MmResult<String> {
    // Parse the template into an AST
    let ast = parser::parse_template(template)?;
    // Evaluate the AST against the context
    evaluate(&ast, ctx)
}

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::{MediaClass, MediaFormat, MediaGroup, MediaQuality};
    use std::path::PathBuf;

    /// Helper: build a TagMap from key-value pairs
    fn make_tags(pairs: &[(&str, &str)]) -> TagMap {
        let mut map = TagMap::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), vec![v.to_string()]);
        }
        map
    }

    /// Helper: build a TagMap with multi-value tags
    fn make_multi_tags(pairs: &[(&str, &[&str])]) -> TagMap {
        let mut map = TagMap::new();
        for (k, vals) in pairs {
            map.insert(
                k.to_string(),
                vals.iter().map(std::string::ToString::to_string).collect(),
            );
        }
        map
    }

    // ── Basic evaluation ────────────────────────────────────────────

    /// Literal-only template returns unchanged
    #[test]
    fn literal_only() {
        let tags = TagMap::new();
        let ctx = EvalContext::new(&tags);
        let result = evaluate_template("Music/Unsorted", &ctx).unwrap();
        assert_eq!(result, "Music/Unsorted");
    }

    /// Single tag resolved from tags map
    #[test]
    fn single_tag_resolved() {
        let tags = make_tags(&[("artist", "Radiohead")]);
        let ctx = EvalContext::new(&tags);
        let result = evaluate_template("<Artist>", &ctx).unwrap();
        assert_eq!(result, "Radiohead");
    }

    /// Tag + literal + tag sequence
    #[test]
    fn tag_literal_tag() {
        let tags = make_tags(&[("artist", "Radiohead"), ("album", "OK Computer")]);
        let ctx = EvalContext::new(&tags);
        let result = evaluate_template("<Artist>/<Album>", &ctx).unwrap();
        assert_eq!(result, "Radiohead/OK Computer");
    }

    // ── Missing tag modes ───────────────────────────────────────────

    /// Missing tag with Empty mode returns empty string
    #[test]
    fn missing_tag_empty_mode() {
        let tags = TagMap::new();
        let ctx = EvalContext::new(&tags).with_missing_tag_mode(MissingTagMode::Empty);
        let result = evaluate_template("<Artist>", &ctx).unwrap();
        assert_eq!(result, "");
    }

    /// Missing tag with Literal mode returns bracketed name
    #[test]
    fn missing_tag_literal_mode() {
        let tags = TagMap::new();
        let ctx = EvalContext::new(&tags).with_missing_tag_mode(MissingTagMode::Literal);
        let result = evaluate_template("<Artist>", &ctx).unwrap();
        assert_eq!(result, "<Artist>");
    }

    /// Missing tag with Error mode returns error
    #[test]
    fn missing_tag_error_mode() {
        let tags = TagMap::new();
        let ctx = EvalContext::new(&tags).with_missing_tag_mode(MissingTagMode::Error);
        let result = evaluate_template("<Artist>", &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing tag"));
    }

    // ── Case-insensitive tag lookup ─────────────────────────────────

    /// Tags are resolved case-insensitively
    #[test]
    fn case_insensitive_tag() {
        let tags = make_tags(&[("artist", "Björk")]);
        let ctx = EvalContext::new(&tags);
        // Template uses "ARTIST" but tag map has "artist"
        let result = evaluate_template("<ARTIST>", &ctx).unwrap();
        assert_eq!(result, "Björk");
    }

    // ── Multi-value tags ────────────────────────────────────────────

    /// Multi-value tag in path mode returns first value only
    #[test]
    fn multi_value_path_mode() {
        let tags = make_multi_tags(&[("artist", &["Artist A", "Artist B"])]);
        let ctx = EvalContext::new(&tags).with_path_mode(true);
        let result = evaluate_template("<Artist>", &ctx).unwrap();
        assert_eq!(result, "Artist A");
    }

    /// Multi-value tag in display mode returns all values joined
    #[test]
    fn multi_value_display_mode() {
        let tags = make_multi_tags(&[("artist", &["Artist A", "Artist B"])]);
        let ctx = EvalContext::new(&tags).with_path_mode(false);
        let result = evaluate_template("<Artist>", &ctx).unwrap();
        assert_eq!(result, "Artist A; Artist B");
    }

    // ── Virtual tags ────────────────────────────────────────────────

    /// Virtual tag: Filename
    #[test]
    fn virtual_filename() {
        let tags = TagMap::new();
        let path = PathBuf::from("/music/song.mp3");
        let ctx = EvalContext::new(&tags).with_file_path(&path);
        let result = evaluate_template("<Filename>", &ctx).unwrap();
        assert_eq!(result, "song");
    }

    /// Virtual tag: Extension
    #[test]
    fn virtual_extension() {
        let tags = TagMap::new();
        let path = PathBuf::from("/music/song.flac");
        let ctx = EvalContext::new(&tags).with_file_path(&path);
        let result = evaluate_template("<Extension>", &ctx).unwrap();
        assert_eq!(result, "flac");
    }

    /// Virtual tag: Folder
    #[test]
    fn virtual_folder() {
        let tags = TagMap::new();
        let path = PathBuf::from("/music/Rock/song.mp3");
        let ctx = EvalContext::new(&tags).with_file_path(&path);
        let result = evaluate_template("<Folder>", &ctx).unwrap();
        assert_eq!(result, "Rock");
    }

    /// Virtual tag: Duration
    #[test]
    fn virtual_duration() {
        let tags = TagMap::new();
        let props = AudioProperties {
            duration_secs: 222.5,
            bitrate_kbps: Some(320),
            sample_rate_hz: Some(44100),
            channels: Some(2),
            bits_per_sample: Some(16),
        };
        let ctx = EvalContext::new(&tags).with_audio_props(&props);
        let result = evaluate_template("<Duration>", &ctx).unwrap();
        assert_eq!(result, "3:42");
    }

    /// Virtual tag: Bitrate
    #[test]
    fn virtual_bitrate() {
        let tags = TagMap::new();
        let props = AudioProperties {
            duration_secs: 180.0,
            bitrate_kbps: Some(320),
            sample_rate_hz: None,
            channels: None,
            bits_per_sample: None,
        };
        let ctx = EvalContext::new(&tags).with_audio_props(&props);
        let result = evaluate_template("<Bitrate>", &ctx).unwrap();
        assert_eq!(result, "320");
    }

    /// Virtual tag: MediaClass
    #[test]
    fn virtual_media_class() {
        let tags = TagMap::new();
        let class = MediaClassification::new(
            MediaGroup::Audio,
            MediaFormat::FLAC,
            MediaClass::Music,
            MediaQuality::Lossless,
        );
        let ctx = EvalContext::new(&tags).with_classification(&class);
        let result = evaluate_template("<Media Class>", &ctx).unwrap();
        assert_eq!(result, "Music");
    }

    // ── Function calls in templates ─────────────────────────────────

    /// Simple function call: $Upper(<Artist>)
    #[test]
    fn func_upper_tag() {
        let tags = make_tags(&[("artist", "radiohead")]);
        let ctx = EvalContext::new(&tags);
        let result = evaluate_template("$Upper(<Artist>)", &ctx).unwrap();
        assert_eq!(result, "RADIOHEAD");
    }

    /// Nested function: $If($IsNull(<Artist>),"Unknown",<Artist>)
    #[test]
    fn nested_if_isnull() {
        // With artist
        let tags = make_tags(&[("artist", "Björk")]);
        let ctx = EvalContext::new(&tags);
        let result =
            evaluate_template("$If($IsNull(<Artist>),\"Unknown\",<Artist>)", &ctx).unwrap();
        assert_eq!(result, "Björk");

        // Without artist
        let empty_tags = TagMap::new();
        let ctx = EvalContext::new(&empty_tags);
        let result =
            evaluate_template("$If($IsNull(<Artist>),\"Unknown\",<Artist>)", &ctx).unwrap();
        assert_eq!(result, "Unknown");
    }

    /// $Pad for track numbers
    #[test]
    fn pad_track_number() {
        let tags = make_tags(&[("track_number", "3")]);
        let ctx = EvalContext::new(&tags);
        let result = evaluate_template("$Pad(<Track#>,\"2\")", &ctx).unwrap();
        assert_eq!(result, "03");
    }

    // ── Sequence with functions ─────────────────────────────────────

    /// Full template: <Artist>/<Album>/$Pad(<Track#>,"2") - <Title>
    #[test]
    fn full_template() {
        let tags = make_tags(&[
            ("artist", "Pink Floyd"),
            ("album", "The Wall"),
            ("track_number", "5"),
            ("title", "Another Brick in the Wall"),
        ]);
        let ctx = EvalContext::new(&tags);
        let result =
            evaluate_template("<Artist>/<Album>/$Pad(<Track#>,\"2\") - <Title>", &ctx).unwrap();
        assert_eq!(result, "Pink Floyd/The Wall/05 - Another Brick in the Wall");
    }

    /// evaluate_template end-to-end
    #[test]
    fn evaluate_template_end_to_end() {
        let tags = make_tags(&[("album_artist", "VA"), ("album", "Now 100")]);
        let ctx = EvalContext::new(&tags);
        let result = evaluate_template("<Album Artist>/<Album>", &ctx).unwrap();
        assert_eq!(result, "VA/Now 100");
    }

    // ── Legacy syntax ───────────────────────────────────────────────

    /// Legacy {key} syntax does not error (passes through as literal)
    #[test]
    fn legacy_syntax_passthrough() {
        let tags = TagMap::new();
        let ctx = EvalContext::new(&tags);
        let result = evaluate_template("{artist}/{album}", &ctx).unwrap();
        assert_eq!(result, "{artist}/{album}");
    }

    // ── Error propagation ───────────────────────────────────────────

    /// Invalid function name returns error
    #[test]
    fn invalid_function_error() {
        let tags = TagMap::new();
        let ctx = EvalContext::new(&tags);
        let result = evaluate_template("$BadFunc(\"x\")", &ctx);
        assert!(result.is_err());
    }

    // ── Virtual tags without data ───────────────────────────────────

    /// Virtual tag with no file_path returns empty (Empty mode)
    #[test]
    fn virtual_no_data_empty_mode() {
        let tags = TagMap::new();
        let ctx = EvalContext::new(&tags).with_missing_tag_mode(MissingTagMode::Empty);
        let result = evaluate_template("<Filename>", &ctx).unwrap();
        assert_eq!(result, "");
    }

    // ── Custom tags ─────────────────────────────────────────────────

    /// Custom tag resolved from tags map
    #[test]
    fn custom_tag_resolved() {
        let tags = make_tags(&[("custom_1", "my value")]);
        let ctx = EvalContext::new(&tags);
        let result = evaluate_template("<Custom1>", &ctx).unwrap();
        assert_eq!(result, "my value");
    }

    // ── $Date in a sequence ─────────────────────────────────────────

    /// $Date produces output in a sequence
    #[test]
    fn date_in_sequence() {
        let tags = make_tags(&[("artist", "Test")]);
        let ctx = EvalContext::new(&tags);
        let result = evaluate_template("<Artist>/$Date(\"%Y\")", &ctx).unwrap();
        // Should start with "Test/" followed by a 4-digit year
        assert!(result.starts_with("Test/"));
        assert!(result.len() >= 9); // "Test/YYYY"
    }

    // ── Duration formatting ─────────────────────────────────────────

    /// Duration with hours formats as H:MM:SS
    #[test]
    fn duration_with_hours() {
        let tags = TagMap::new();
        let props = AudioProperties {
            duration_secs: 3723.0, // 1:02:03
            bitrate_kbps: None,
            sample_rate_hz: None,
            channels: None,
            bits_per_sample: None,
        };
        let ctx = EvalContext::new(&tags).with_audio_props(&props);
        let result = evaluate_template("<Duration>", &ctx).unwrap();
        assert_eq!(result, "1:02:03");
    }

    // ── Builder pattern ─────────────────────────────────────────────

    /// Builder pattern constructs context correctly
    #[test]
    fn builder_pattern() {
        let tags = make_tags(&[("artist", "Test")]);
        let path = PathBuf::from("/music/test.mp3");
        let props = AudioProperties {
            duration_secs: 180.0,
            bitrate_kbps: Some(320),
            sample_rate_hz: Some(44100),
            channels: Some(2),
            bits_per_sample: Some(16),
        };
        let class = MediaClassification::new(
            MediaGroup::Audio,
            MediaFormat::MP3,
            MediaClass::Music,
            MediaQuality::Lossy320,
        );
        let ctx = EvalContext::new(&tags)
            .with_file_path(&path)
            .with_audio_props(&props)
            .with_classification(&class)
            .with_path_mode(true)
            .with_missing_tag_mode(MissingTagMode::Empty);
        // Just verify it compiles and runs
        let result = evaluate_template("<Artist>", &ctx).unwrap();
        assert_eq!(result, "Test");
    }

    /// MeedyaMeta custom tag
    #[test]
    fn meedyameta_tag() {
        let tags = make_tags(&[("meedyameta.spotifyid", "abc123")]);
        let ctx = EvalContext::new(&tags);
        let result = evaluate_template("<MeedyaMeta.SpotifyId>", &ctx).unwrap();
        assert_eq!(result, "abc123");
    }
}
