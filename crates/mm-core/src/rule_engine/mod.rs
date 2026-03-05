// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Rule Engine (MusicBee-Inspired Template System)
//
// This module implements a full template language for media file renaming
// with the following pipeline:
//
//   1. **Lexer** (`lexer.rs`) — tokenizes `<Tag>`, `$Func()`, literals
//   2. **Parser** (`parser.rs`) — recursive descent → AST with 50-level depth guard
//   3. **Tag Registry** (`tag_registry.rs`) — 40+ bidirectional tag name mappings
//   4. **Functions** (`functions.rs`) — 24 template function implementations
//   5. **Evaluator** (`evaluator.rs`) — AST evaluation against metadata context
//
// Additionally, this module defines the **Rule System**: a declarative
// condition/template system that selects templates based on media properties.
//
// Public API:
//   - `evaluate_template()` — parse + evaluate a single template string
//   - `apply_rules()` — evaluate a rule set against a media file context
//   - `EvalContext` — evaluation context (tags, audio props, classification)
//   - `Rule`, `Condition`, `ConditionOp`, `ConditionMode` — rule system types
//   - `Node`, `Token`, `TagKind`, `VirtualTag` — AST and registry types
//
// License: GPL-2.0-or-later

use serde::{Deserialize, Serialize};

use crate::error::{MmError, MmResult};

// ───────────────────────────────────────────────────────────────────────────
// Submodules
// ───────────────────────────────────────────────────────────────────────────

/// Template tokenizer — converts raw template strings into token streams
pub mod lexer;
/// Recursive descent parser — builds AST from token streams
pub mod parser;
/// Bidirectional tag name ↔ canonical key mappings
pub mod tag_registry;
/// Template function implementations (24 functions)
pub mod functions;
/// AST evaluator and EvalContext
pub mod evaluator;

// ───────────────────────────────────────────────────────────────────────────
// Re-exports — the public API surface
// ───────────────────────────────────────────────────────────────────────────

pub use evaluator::{EvalContext, MissingTagMode, evaluate, evaluate_template};
pub use lexer::Token;
pub use parser::{Node, parse_template, detect_legacy_syntax};
pub use tag_registry::{TagKind, VirtualTag, lookup_tag};

// ───────────────────────────────────────────────────────────────────────────
// Rule system types
// ───────────────────────────────────────────────────────────────────────────

/// A naming rule that conditionally applies a template to matching files.
///
/// Rules are evaluated in priority order (lower `priority` = evaluated first).
/// The first rule whose conditions pass is used.  If `stop_on_match` is true,
/// no further rules are evaluated after this one matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    /// Human-readable name for this rule (e.g. "Music files", "TV Shows")
    pub name: String,
    /// Priority order — lower values are evaluated first (0 = highest priority)
    #[serde(default)]
    pub priority: u32,
    /// Whether this rule is active
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Conditions that must be met for this rule to apply
    #[serde(default)]
    pub conditions: Vec<Condition>,
    /// How to combine conditions: All (AND) or Any (OR)
    #[serde(default)]
    pub condition_mode: ConditionMode,
    /// The template string to evaluate when this rule matches
    pub template: String,
    /// If true, stop evaluating further rules after this one matches
    #[serde(default)]
    pub stop_on_match: bool,
}

/// Default for `enabled` field — rules are enabled by default
fn default_true() -> bool {
    true
}

/// A single condition that tests a tag value against an expected value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Condition {
    /// The tag name to test (e.g. "genre", "mediaclass", "artist")
    pub field: String,
    /// The comparison operator
    pub operator: ConditionOp,
    /// The value to compare against (unused for IsEmpty/IsNotEmpty)
    #[serde(default)]
    pub value: String,
}

/// Comparison operators for rule conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionOp {
    /// Tag value equals the expected value (case-insensitive)
    Equals,
    /// Tag value does not equal the expected value (case-insensitive)
    NotEquals,
    /// Tag value contains the expected substring (case-insensitive)
    Contains,
    /// Tag value does not contain the expected substring (case-insensitive)
    NotContains,
    /// Tag value starts with the expected prefix (case-insensitive)
    StartsWith,
    /// Tag value ends with the expected suffix (case-insensitive)
    EndsWith,
    /// Tag value matches the expected regex pattern
    Matches,
    /// Tag is empty or absent
    IsEmpty,
    /// Tag is non-empty and present
    IsNotEmpty,
}

/// How to combine multiple conditions within a single rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ConditionMode {
    /// All conditions must match (logical AND) — default
    #[default]
    All,
    /// Any condition must match (logical OR)
    Any,
}

// ───────────────────────────────────────────────────────────────────────────
// Condition evaluation
// ───────────────────────────────────────────────────────────────────────────

/// Evaluate a single condition against the evaluation context.
///
/// Resolves the condition's field from the context (using the tag registry)
/// and applies the comparison operator.
fn evaluate_condition(condition: &Condition, ctx: &EvalContext<'_>) -> MmResult<bool> {
    // Resolve the tag value from the context
    let tag_value = ctx.resolve_tag(&condition.field)?;
    let tag_lower = tag_value.to_lowercase();
    let expected_lower = condition.value.to_lowercase();

    // Apply the operator
    match condition.operator {
        ConditionOp::Equals => Ok(tag_lower == expected_lower),
        ConditionOp::NotEquals => Ok(tag_lower != expected_lower),
        ConditionOp::Contains => Ok(tag_lower.contains(&expected_lower)),
        ConditionOp::NotContains => Ok(!tag_lower.contains(&expected_lower)),
        ConditionOp::StartsWith => Ok(tag_lower.starts_with(&expected_lower)),
        ConditionOp::EndsWith => Ok(tag_lower.ends_with(&expected_lower)),
        ConditionOp::Matches => {
            // Compile regex (cached) and test against the tag value
            let re = regex::Regex::new(&condition.value).map_err(|e| {
                MmError::RuleEngine(format!(
                    "invalid regex in condition for '{}': {e}",
                    condition.field
                ))
            })?;
            Ok(re.is_match(&tag_value))
        }
        ConditionOp::IsEmpty => Ok(tag_value.is_empty()),
        ConditionOp::IsNotEmpty => Ok(!tag_value.is_empty()),
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Rule evaluation
// ───────────────────────────────────────────────────────────────────────────

/// Evaluate a single rule against the context.
///
/// Returns `Some(evaluated_path)` if all/any conditions pass and the
/// template evaluates successfully.  Returns `None` if conditions don't match.
/// Returns `Err` if template evaluation fails.
pub fn evaluate_rule(rule: &Rule, ctx: &EvalContext<'_>) -> MmResult<Option<String>> {
    // Skip disabled rules
    if !rule.enabled {
        return Ok(None);
    }

    // Evaluate conditions
    let conditions_pass = if rule.conditions.is_empty() {
        // No conditions = always matches
        true
    } else {
        match rule.condition_mode {
            ConditionMode::All => {
                // All conditions must pass
                let mut all_pass = true;
                for cond in &rule.conditions {
                    if !evaluate_condition(cond, ctx)? {
                        all_pass = false;
                        break;
                    }
                }
                all_pass
            }
            ConditionMode::Any => {
                // Any condition must pass
                let mut any_pass = false;
                for cond in &rule.conditions {
                    if evaluate_condition(cond, ctx)? {
                        any_pass = true;
                        break;
                    }
                }
                any_pass
            }
        }
    };

    // If conditions don't pass, this rule doesn't apply
    if !conditions_pass {
        return Ok(None);
    }

    // Conditions passed — evaluate the template
    let result = evaluate_template(&rule.template, ctx)?;
    Ok(Some(result))
}

/// Evaluate a set of rules against the context, returning the first match.
///
/// Rules are sorted by priority (ascending — lower value = higher priority).
/// The first rule whose conditions pass is evaluated and its result returned.
/// If a matching rule has `stop_on_match = true`, evaluation stops immediately.
///
/// Returns `None` if no rules match.
pub fn apply_rules(rules: &[Rule], ctx: &EvalContext<'_>) -> MmResult<Option<String>> {
    // Sort rules by priority (stable sort preserves insertion order for equal priorities)
    let mut sorted: Vec<&Rule> = rules.iter().collect();
    sorted.sort_by_key(|r| r.priority);

    // Evaluate rules in priority order
    for rule in sorted {
        if let Some(result) = evaluate_rule(rule, ctx)? {
            return Ok(Some(result));
        }
    }

    // No rules matched
    Ok(None)
}

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::{MediaClass, MediaClassification, MediaFormat, MediaGroup, MediaQuality};
    use crate::metadata::TagMap;

    /// Helper: build a TagMap from key-value pairs
    fn make_tags(pairs: &[(&str, &str)]) -> TagMap {
        let mut map = TagMap::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), vec![v.to_string()]);
        }
        map
    }

    // ── Rule type tests ─────────────────────────────────────────────

    /// Rule derives Debug and Clone
    #[test]
    fn rule_debug_clone() {
        let rule = Rule {
            name: "Test".into(),
            priority: 0,
            enabled: true,
            conditions: vec![],
            condition_mode: ConditionMode::All,
            template: "<Artist>".into(),
            stop_on_match: false,
        };
        let cloned = rule.clone();
        assert_eq!(format!("{:?}", rule), format!("{:?}", cloned));
    }

    /// Rule serializes and deserializes
    #[test]
    fn rule_serde_roundtrip() {
        let rule = Rule {
            name: "Music".into(),
            priority: 10,
            enabled: true,
            conditions: vec![Condition {
                field: "mediaclass".into(),
                operator: ConditionOp::Equals,
                value: "Music".into(),
            }],
            condition_mode: ConditionMode::All,
            template: "<Artist>/<Album>/<Title>".into(),
            stop_on_match: true,
        };
        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: Rule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule, deserialized);
    }

    /// ConditionMode defaults to All
    #[test]
    fn condition_mode_default() {
        let mode = ConditionMode::default();
        assert_eq!(mode, ConditionMode::All);
    }

    // ── Condition evaluation ────────────────────────────────────────

    /// ConditionOp::Equals matches (case-insensitive)
    #[test]
    fn condition_equals() {
        let tags = make_tags(&[("genre", "Rock")]);
        let ctx = EvalContext::new(&tags);
        let cond = Condition {
            field: "Genre".into(),
            operator: ConditionOp::Equals,
            value: "rock".into(),
        };
        assert!(evaluate_condition(&cond, &ctx).unwrap());
    }

    /// ConditionOp::NotEquals
    #[test]
    fn condition_not_equals() {
        let tags = make_tags(&[("genre", "Rock")]);
        let ctx = EvalContext::new(&tags);
        let cond = Condition {
            field: "Genre".into(),
            operator: ConditionOp::NotEquals,
            value: "Pop".into(),
        };
        assert!(evaluate_condition(&cond, &ctx).unwrap());
    }

    /// ConditionOp::Contains
    #[test]
    fn condition_contains() {
        let tags = make_tags(&[("genre", "Progressive Rock")]);
        let ctx = EvalContext::new(&tags);
        let cond = Condition {
            field: "Genre".into(),
            operator: ConditionOp::Contains,
            value: "rock".into(),
        };
        assert!(evaluate_condition(&cond, &ctx).unwrap());
    }

    /// ConditionOp::NotContains
    #[test]
    fn condition_not_contains() {
        let tags = make_tags(&[("genre", "Rock")]);
        let ctx = EvalContext::new(&tags);
        let cond = Condition {
            field: "Genre".into(),
            operator: ConditionOp::NotContains,
            value: "jazz".into(),
        };
        assert!(evaluate_condition(&cond, &ctx).unwrap());
    }

    /// ConditionOp::StartsWith
    #[test]
    fn condition_starts_with() {
        let tags = make_tags(&[("artist", "The Beatles")]);
        let ctx = EvalContext::new(&tags);
        let cond = Condition {
            field: "Artist".into(),
            operator: ConditionOp::StartsWith,
            value: "the".into(),
        };
        assert!(evaluate_condition(&cond, &ctx).unwrap());
    }

    /// ConditionOp::EndsWith
    #[test]
    fn condition_ends_with() {
        let tags = make_tags(&[("title", "Song (Live)")]);
        let ctx = EvalContext::new(&tags);
        let cond = Condition {
            field: "Title".into(),
            operator: ConditionOp::EndsWith,
            value: "(live)".into(),
        };
        assert!(evaluate_condition(&cond, &ctx).unwrap());
    }

    /// ConditionOp::Matches (regex)
    #[test]
    fn condition_matches_regex() {
        let tags = make_tags(&[("track_number", "5")]);
        let ctx = EvalContext::new(&tags);
        let cond = Condition {
            field: "Track#".into(),
            operator: ConditionOp::Matches,
            value: r"^\d+$".into(),
        };
        assert!(evaluate_condition(&cond, &ctx).unwrap());
    }

    /// ConditionOp::Matches with invalid regex returns error
    #[test]
    fn condition_matches_invalid_regex() {
        let tags = make_tags(&[("title", "test")]);
        let ctx = EvalContext::new(&tags);
        let cond = Condition {
            field: "Title".into(),
            operator: ConditionOp::Matches,
            value: r"[invalid".into(),
        };
        assert!(evaluate_condition(&cond, &ctx).is_err());
    }

    /// ConditionOp::IsEmpty
    #[test]
    fn condition_is_empty() {
        let tags = TagMap::new(); // no artist tag
        let ctx = EvalContext::new(&tags);
        let cond = Condition {
            field: "Artist".into(),
            operator: ConditionOp::IsEmpty,
            value: String::new(),
        };
        assert!(evaluate_condition(&cond, &ctx).unwrap());
    }

    /// ConditionOp::IsNotEmpty
    #[test]
    fn condition_is_not_empty() {
        let tags = make_tags(&[("artist", "Radiohead")]);
        let ctx = EvalContext::new(&tags);
        let cond = Condition {
            field: "Artist".into(),
            operator: ConditionOp::IsNotEmpty,
            value: String::new(),
        };
        assert!(evaluate_condition(&cond, &ctx).unwrap());
    }

    // ── ConditionMode evaluation ────────────────────────────────────

    /// ConditionMode::All — all must pass
    #[test]
    fn mode_all_passes() {
        let tags = make_tags(&[("genre", "Rock"), ("artist", "Radiohead")]);
        let ctx = EvalContext::new(&tags);
        let rule = Rule {
            name: "Test".into(),
            priority: 0,
            enabled: true,
            conditions: vec![
                Condition {
                    field: "Genre".into(),
                    operator: ConditionOp::Equals,
                    value: "Rock".into(),
                },
                Condition {
                    field: "Artist".into(),
                    operator: ConditionOp::IsNotEmpty,
                    value: String::new(),
                },
            ],
            condition_mode: ConditionMode::All,
            template: "<Artist>".into(),
            stop_on_match: false,
        };
        let result = evaluate_rule(&rule, &ctx).unwrap();
        assert_eq!(result, Some("Radiohead".into()));
    }

    /// ConditionMode::All — one fails
    #[test]
    fn mode_all_one_fails() {
        let tags = make_tags(&[("genre", "Pop")]);
        let ctx = EvalContext::new(&tags);
        let rule = Rule {
            name: "Test".into(),
            priority: 0,
            enabled: true,
            conditions: vec![Condition {
                field: "Genre".into(),
                operator: ConditionOp::Equals,
                value: "Rock".into(),
            }],
            condition_mode: ConditionMode::All,
            template: "<Genre>".into(),
            stop_on_match: false,
        };
        let result = evaluate_rule(&rule, &ctx).unwrap();
        assert_eq!(result, None);
    }

    /// ConditionMode::Any — one passes
    #[test]
    fn mode_any_one_passes() {
        let tags = make_tags(&[("genre", "Jazz")]);
        let ctx = EvalContext::new(&tags);
        let rule = Rule {
            name: "Test".into(),
            priority: 0,
            enabled: true,
            conditions: vec![
                Condition {
                    field: "Genre".into(),
                    operator: ConditionOp::Equals,
                    value: "Rock".into(),
                },
                Condition {
                    field: "Genre".into(),
                    operator: ConditionOp::Equals,
                    value: "Jazz".into(),
                },
            ],
            condition_mode: ConditionMode::Any,
            template: "<Genre>".into(),
            stop_on_match: false,
        };
        let result = evaluate_rule(&rule, &ctx).unwrap();
        assert_eq!(result, Some("Jazz".into()));
    }

    /// ConditionMode::Any — all fail
    #[test]
    fn mode_any_all_fail() {
        let tags = make_tags(&[("genre", "Classical")]);
        let ctx = EvalContext::new(&tags);
        let rule = Rule {
            name: "Test".into(),
            priority: 0,
            enabled: true,
            conditions: vec![
                Condition {
                    field: "Genre".into(),
                    operator: ConditionOp::Equals,
                    value: "Rock".into(),
                },
                Condition {
                    field: "Genre".into(),
                    operator: ConditionOp::Equals,
                    value: "Jazz".into(),
                },
            ],
            condition_mode: ConditionMode::Any,
            template: "<Genre>".into(),
            stop_on_match: false,
        };
        let result = evaluate_rule(&rule, &ctx).unwrap();
        assert_eq!(result, None);
    }

    // ── Rule evaluation ─────────────────────────────────────────────

    /// Disabled rule is skipped
    #[test]
    fn disabled_rule_skipped() {
        let tags = make_tags(&[("artist", "Test")]);
        let ctx = EvalContext::new(&tags);
        let rule = Rule {
            name: "Disabled".into(),
            priority: 0,
            enabled: false,
            conditions: vec![],
            condition_mode: ConditionMode::All,
            template: "<Artist>".into(),
            stop_on_match: false,
        };
        let result = evaluate_rule(&rule, &ctx).unwrap();
        assert_eq!(result, None);
    }

    /// Rule with no conditions always matches
    #[test]
    fn no_conditions_always_matches() {
        let tags = make_tags(&[("artist", "Test")]);
        let ctx = EvalContext::new(&tags);
        let rule = Rule {
            name: "Catch-all".into(),
            priority: 100,
            enabled: true,
            conditions: vec![],
            condition_mode: ConditionMode::All,
            template: "<Artist>".into(),
            stop_on_match: false,
        };
        let result = evaluate_rule(&rule, &ctx).unwrap();
        assert_eq!(result, Some("Test".into()));
    }

    // ── apply_rules ─────────────────────────────────────────────────

    /// Priority ordering: lower priority value wins
    #[test]
    fn priority_ordering() {
        let tags = make_tags(&[("artist", "Test"), ("genre", "Rock")]);
        let ctx = EvalContext::new(&tags);
        let rules = vec![
            Rule {
                name: "Low priority".into(),
                priority: 100,
                enabled: true,
                conditions: vec![],
                condition_mode: ConditionMode::All,
                template: "\"fallback\"".into(),
                stop_on_match: false,
            },
            Rule {
                name: "High priority".into(),
                priority: 1,
                enabled: true,
                conditions: vec![],
                condition_mode: ConditionMode::All,
                template: "<Artist>".into(),
                stop_on_match: false,
            },
        ];
        let result = apply_rules(&rules, &ctx).unwrap();
        assert_eq!(result, Some("Test".into()));
    }

    /// Empty rule set returns None
    #[test]
    fn empty_rules_returns_none() {
        let tags = TagMap::new();
        let ctx = EvalContext::new(&tags);
        let result = apply_rules(&[], &ctx).unwrap();
        assert_eq!(result, None);
    }

    /// All non-matching rules return None
    #[test]
    fn no_matching_rules() {
        let tags = make_tags(&[("genre", "Classical")]);
        let ctx = EvalContext::new(&tags);
        let rules = vec![Rule {
            name: "Rock only".into(),
            priority: 0,
            enabled: true,
            conditions: vec![Condition {
                field: "Genre".into(),
                operator: ConditionOp::Equals,
                value: "Rock".into(),
            }],
            condition_mode: ConditionMode::All,
            template: "<Genre>".into(),
            stop_on_match: false,
        }];
        let result = apply_rules(&rules, &ctx).unwrap();
        assert_eq!(result, None);
    }

    /// Full integration: rule with media class condition
    #[test]
    fn integration_media_class_rule() {
        let tags = make_tags(&[
            ("artist", "Pink Floyd"),
            ("album", "The Dark Side of the Moon"),
            ("title", "Time"),
            ("track_number", "4"),
        ]);
        let class = MediaClassification::new(
            MediaGroup::Audio,
            MediaFormat::FLAC,
            MediaClass::Music,
            MediaQuality::Lossless,
        );
        let ctx = EvalContext::new(&tags).with_classification(&class);
        let rules = vec![
            Rule {
                name: "Music".into(),
                priority: 10,
                enabled: true,
                conditions: vec![Condition {
                    field: "MediaClass".into(),
                    operator: ConditionOp::Equals,
                    value: "Music".into(),
                }],
                condition_mode: ConditionMode::All,
                template: "<Artist>/<Album>/$Pad(<Track#>,\"2\") - <Title>".into(),
                stop_on_match: true,
            },
            Rule {
                name: "Fallback".into(),
                priority: 100,
                enabled: true,
                conditions: vec![],
                condition_mode: ConditionMode::All,
                template: "Unsorted/<Filename>".into(),
                stop_on_match: false,
            },
        ];
        let result = apply_rules(&rules, &ctx).unwrap();
        assert_eq!(
            result,
            Some("Pink Floyd/The Dark Side of the Moon/04 - Time".into())
        );
    }
}
