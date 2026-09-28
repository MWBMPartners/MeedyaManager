// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — conformance test for policy MWBM-MEDIA-LANG 1.0.0
// (docs/standards/media-language-bcp47-policy.md). This is MeedyaManager's
// own copy of the check every project implementing this policy runs — see
// section 8.1 of the policy document — modelled directly on the master
// copy of this test, `crates/meedya-lang/tests/conformance.rs` in
// MeedyaSuite-core (read it before changing this file).
//
// MeedyaManager's profile (policy section 2) is "canonical" only today: it
// edits stored metadata, and has no language menus yet (that would be the
// "presentation" profile, once one exists). So of the thirteen sections
// the policy's own table (8.1) lists, this test runs the four the
// "canonical" profile needs for the parts of it MeedyaManager actually
// implements: `canonicalise` (LANG-001, LANG-026), `legacy_three_letter`
// (LANG-002, LANG-003), `iso639_2_write` (TRACK-070), and
// `canonical_order` (LANG-010 to LANG-027, used here for LANG-002's own
// "canonical form is stable" style checks and for anything that later
// needs to order languages — MeedyaManager does not sort tracks or
// translations today, but the policy lists `canonical_order` as needed by
// the "canonical" profile generally, not only by a project that currently
// has multiple items to sort). `track_order`, `sidecar_name` and
// `posix_locale` are NOT run here: MeedyaManager does not reorder tracks
// within a container (that is MeedyaConverter's job), does not yet read or
// name sidecar subtitle/lyric files by language (see the tracking issue
// for that gap), and does not convert operating-system locale names
// anywhere in its own code today.
//
// Policy 8.1 is explicit that a conformance harness must FAIL — never
// quietly pass — on a section name it does not recognise at all, on one of
// the sections it actually needs being missing or empty, or on a case
// missing a field its own schema requires. This test enforces all three,
// even though it only RUNS four of the thirteen sections: the "does not
// recognise a section name" check below still checks every key in the
// fixture file against the FULL list of thirteen names the policy defines
// (`ALL_POLICY_SECTIONS`), not just the four this test happens to run —
// finding a stray fourteenth name is exactly the situation this check
// exists for, and narrowing that list to "sections we use" would silently
// stop catching it.

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use meedya_lang::{canonicalise, embedded_data_version, from_legacy_three_letter, iso639_2_write};

// ---------------------------------------------------------------------------
// Fixture-shape robustness helpers (policy 8.1) — identical in spirit to
// the master copy's `require_present` / `case_id_hint`; kept here rather
// than shared because the master test lives in a different crate this one
// cannot depend on.
// ---------------------------------------------------------------------------

/// Panics unless every one of `keys` is present (as a JSON object key —
/// `null` counts as present, only a missing key does not) on `case`. An
/// `Option<T>` struct field cannot make this distinction on its own: a
/// case that OMITS a required-but-nullable field would deserialise into
/// exactly the same Rust value (`None`) as one that explicitly writes
/// `"field": null` — precisely the ambiguity policy 8.1 requires the
/// harness to resolve, not paper over.
fn require_present(case: &Value, keys: &[&str], id_hint: &str) {
    let obj = case
        .as_object()
        .unwrap_or_else(|| panic!("{id_hint}: fixture case is not a JSON object"));
    for key in keys {
        assert!(
            obj.contains_key(*key),
            "{id_hint}: fixture case is missing required field {key:?}"
        );
    }
}

fn case_id_hint(case: &Value, section: &str) -> String {
    case.get("id")
        .and_then(Value::as_str)
        .map(|id| format!("{section}/{id}"))
        .unwrap_or_else(|| format!("{section}/<no id>"))
}

/// Every section name the policy's own table (8.1) defines, in the order
/// that table lists them — the full set of thirteen, not just the four
/// this test runs. Checked against so a section this harness has never
/// heard of at all is a hard failure (a genuinely new, fourteenth section
/// name would also fail here, which is the correct behaviour: it means the
/// policy grew a section this copy of the test has not been taught about
/// yet, and that is exactly the kind of drift policy 8.1 exists to catch).
const ALL_POLICY_SECTIONS: &[&str] = &[
    "canonicalise",
    "legacy_three_letter",
    "iso639_2_write",
    "sidecar_name",
    "posix_locale",
    "canonical_order",
    "track_order",
    "presentation_order",
    "subtitle_menu",
    "label",
    "match",
    "auto_select_audio",
    "auto_select_subtitle",
];

/// The sections MeedyaManager's "canonical" profile actually needs today
/// — see the module doc comment above for why each one is or is not here.
/// A subset of [`ALL_POLICY_SECTIONS`]; every failure mode required for the
/// FULL list of thirteen ("unknown section name") is still checked against
/// all thirteen, but "needed section missing or empty" is checked only
/// against this narrower list, because a section MeedyaManager does not
/// use is allowed to be absent from a future fixture update without this
/// test breaking for no reason.
const NEEDED_SECTIONS: &[&str] = &[
    "canonicalise",
    "legacy_three_letter",
    "iso639_2_write",
    "canonical_order",
];

// ---------------------------------------------------------------------------
// Typed fixture shapes for the four sections this test runs. Deliberately
// NOT a struct covering the whole file (unlike the master copy, which
// implements every section its own crate uses) — this test does not need
// to deserialise the nine sections it never runs, and serde would refuse
// to compile a struct field type for a shape (e.g. `label`'s `role_names`
// map) this file has no reason to know about.
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CanonicaliseCase {
    id: String,
    input: String,
    expected: Option<String>,
    kind: String,
}

#[derive(Deserialize)]
struct SimpleCase {
    id: String,
    input: String,
    expected: Option<String>,
}

#[derive(Deserialize)]
struct Iso639WriteExpected {
    b: String,
    t: String,
}

#[derive(Deserialize)]
struct Iso639WriteCase {
    id: String,
    input: String,
    expected: Iso639WriteExpected,
}

#[derive(Deserialize)]
struct OrderItem {
    id: Option<String>,
    tag: String,
    original: Option<bool>,
}

#[derive(Deserialize)]
struct OrderCase {
    id: String,
    items: Vec<OrderItem>,
    expected: Vec<String>,
}

// ---------------------------------------------------------------------------
// A minimal `LanguageItem` for `canonical_order`, matching the master
// test's own wrapper type — MeedyaManager has no track/translation model
// of its own to reuse here, and reaching into `meedya_lang`'s internals
// instead of its public trait would test something a real caller could
// never actually exercise.
// ---------------------------------------------------------------------------

struct OrderTestItem {
    id: String,
    tag: meedya_lang::LanguageTag,
    original: bool,
}

impl meedya_lang::LanguageItem for OrderTestItem {
    fn language(&self) -> &meedya_lang::LanguageTag {
        &self.tag
    }
    fn is_original(&self) -> bool {
        self.original
    }
}

// ---------------------------------------------------------------------------
// The test
// ---------------------------------------------------------------------------

#[test]
fn every_needed_conformance_case_passes() {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/bcp47-language-policy-v1.json"
    ))
    .expect("could not read tests/fixtures/bcp47-language-policy-v1.json");

    let raw_value: Value =
        serde_json::from_str(&raw).expect("fixture file is not valid JSON at all");
    let top_level = raw_value
        .as_object()
        .expect("fixture file's top level is not a JSON object");

    // -- Fail on a section name this harness — or the policy itself, per
    // ALL_POLICY_SECTIONS — has never heard of. -----------------------------
    for key in top_level.keys() {
        if key == "$schema"
            || key == "policy"
            || key == "policy_version"
            || key == "fixtures_version"
            || key == "data_version"
        {
            continue;
        }
        assert!(
            ALL_POLICY_SECTIONS.contains(&key.as_str()),
            "fixture file has a section {key:?} this policy's own section list (8.1) does not \
             name — policy 8.1 requires failing on an unknown section, not silently ignoring it"
        );
    }

    // -- Fail if a section MeedyaManager's profile needs is missing or
    // empty. A section it does NOT need may be absent without failing this
    // test — see NEEDED_SECTIONS's doc comment. -----------------------------
    for section in NEEDED_SECTIONS {
        let arr = top_level
            .get(*section)
            .and_then(Value::as_array)
            .unwrap_or_else(|| {
                panic!(
                    "fixture file has no array section {section:?}, which MeedyaManager's \
                        \"canonical\" profile needs"
                )
            });
        assert!(
            !arr.is_empty(),
            "fixture section {section:?} is empty — policy 8.1 requires failing on an empty \
             section this harness needs, not reporting success having run nothing"
        );
    }

    assert_eq!(
        raw_value["policy"].as_str(),
        Some("MWBM-MEDIA-LANG"),
        "fixture file's own policy identifier does not match"
    );
    assert_eq!(
        raw_value["policy_version"].as_str(),
        Some("1.0.0"),
        "fixture file's own policy version does not match what this test was written against"
    );
    let data_version = raw_value["data_version"]
        .as_str()
        .expect("fixture file has no data_version");
    assert_eq!(
        data_version,
        embedded_data_version(),
        "fixture file was computed against a different reference-data version than the \
         meedya_lang crate this build embeds"
    );

    let canonicalise_cases: Vec<CanonicaliseCase> =
        serde_json::from_value(top_level["canonicalise"].clone())
            .expect("canonicalise section did not match the expected shape");
    let legacy_cases: Vec<SimpleCase> =
        serde_json::from_value(top_level["legacy_three_letter"].clone())
            .expect("legacy_three_letter section did not match the expected shape");
    let write_cases: Vec<Iso639WriteCase> =
        serde_json::from_value(top_level["iso639_2_write"].clone())
            .expect("iso639_2_write section did not match the expected shape");
    let order_cases: Vec<OrderCase> = serde_json::from_value(top_level["canonical_order"].clone())
        .expect("canonical_order section did not match the expected shape");

    let mut failures: Vec<String> = Vec::new();
    let mut counts: HashMap<&'static str, usize> = HashMap::new();

    // -- canonicalise (LANG-001, LANG-026) ----------------------------------
    for (idx, case) in canonicalise_cases.iter().enumerate() {
        require_present(
            &top_level["canonicalise"][idx],
            &["id", "rules", "input", "expected", "kind"],
            &case_id_hint(&top_level["canonicalise"][idx], "canonicalise"),
        );
        *counts.entry("canonicalise").or_default() += 1;

        let got = canonicalise(&case.input);
        let got_kind = match got.kind {
            meedya_lang::TagKind::Ordinary => "ordinary",
            meedya_lang::TagKind::Grandfathered => "grandfathered",
            meedya_lang::TagKind::PrivateUse => "privateuse",
            meedya_lang::TagKind::Malformed => "malformed",
        };
        let got_expected = if got.is_malformed() {
            None
        } else {
            Some(got.tag.clone())
        };
        if got_expected != case.expected || got_kind != case.kind {
            failures.push(format!(
                "{}: canonicalise({:?}) = ({:?}, {:?}), expected ({:?}, {:?})",
                case.id, case.input, got_expected, got_kind, case.expected, case.kind
            ));
        }
    }

    // -- legacy_three_letter (LANG-002, LANG-003) ---------------------------
    for (idx, case) in legacy_cases.iter().enumerate() {
        require_present(
            &top_level["legacy_three_letter"][idx],
            &["id", "rules", "input", "expected"],
            &case_id_hint(
                &top_level["legacy_three_letter"][idx],
                "legacy_three_letter",
            ),
        );
        *counts.entry("legacy_three_letter").or_default() += 1;

        let got = from_legacy_three_letter(&case.input).map(|t| t.tag);
        if got != case.expected {
            failures.push(format!(
                "{}: from_legacy_three_letter({:?}) = {:?}, expected {:?}",
                case.id, case.input, got, case.expected
            ));
        }
    }

    // -- iso639_2_write (TRACK-070) ------------------------------------------
    for (idx, case) in write_cases.iter().enumerate() {
        require_present(
            &top_level["iso639_2_write"][idx],
            &["id", "rules", "input", "expected"],
            &case_id_hint(&top_level["iso639_2_write"][idx], "iso639_2_write"),
        );
        *counts.entry("iso639_2_write").or_default() += 1;

        let tag = canonicalise(&case.input);
        let got = iso639_2_write(&tag);
        if got.bibliographic != case.expected.b || got.terminology != case.expected.t {
            failures.push(format!(
                "{}: iso639_2_write({:?}) = {{b: {:?}, t: {:?}}}, expected {{b: {:?}, t: {:?}}}",
                case.id,
                case.input,
                got.bibliographic,
                got.terminology,
                case.expected.b,
                case.expected.t
            ));
        }
    }

    // -- canonical_order (LANG-010 to LANG-027) ------------------------------
    for (idx, case) in order_cases.iter().enumerate() {
        require_present(
            &top_level["canonical_order"][idx],
            &["id", "rules", "description", "items", "expected"],
            &case_id_hint(&top_level["canonical_order"][idx], "canonical_order"),
        );
        *counts.entry("canonical_order").or_default() += 1;

        let mut items: Vec<OrderTestItem> = case
            .items
            .iter()
            .map(|it| OrderTestItem {
                id: it.id.clone().unwrap_or_else(|| it.tag.clone()),
                tag: canonicalise(&it.tag),
                original: it.original.unwrap_or(false),
            })
            .collect();
        meedya_lang::sort_canonical(&mut items);
        let got: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        if got != case.expected {
            failures.push(format!(
                "{}: canonical order = {:?}, expected {:?}",
                case.id, got, case.expected
            ));
        }
    }

    // -- Every case in every needed section was actually run, and none was
    // silently skipped by a section this harness has never heard of. ------
    let cases_in_file =
        canonicalise_cases.len() + legacy_cases.len() + write_cases.len() + order_cases.len();
    let cases_run: usize = counts.values().sum();
    assert_eq!(
        cases_run, cases_in_file,
        "ran {cases_run} cases but the four needed sections hold {cases_in_file} — a section \
         was skipped"
    );

    println!(
        "MWBM-MEDIA-LANG {}: per-section case counts —",
        raw_value["policy_version"].as_str().unwrap_or("?")
    );
    for section in NEEDED_SECTIONS {
        println!("  {section}: {}", counts.get(section).copied().unwrap_or(0));
    }

    assert!(
        failures.is_empty(),
        "{} of {cases_in_file} conformance cases failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
