// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Language tag handling (policy MWBM-MEDIA-LANG 1.0.0)
//
// This module is MeedyaManager's one and only doorway between the raw
// `language` string tag (see `TAG_LANGUAGE` in `metadata/mod.rs`) and the
// shared `meedya_lang` crate that implements policy MWBM-MEDIA-LANG. Read
// `docs/standards/media-language-bcp47-policy.md` before changing anything
// here — this file deliberately does not restate the policy's rules, only
// cites their IDs, so the two documents can never quietly drift apart.
//
// Policy section 2 assigns MeedyaManager BOTH the "canonical" and
// "presentation" profiles (it edits stored metadata AND shows languages in
// its interfaces) — this module only builds the "canonical" half TODAY,
// not because MeedyaManager is exempt from "presentation" by the policy's
// own table, but simply because MeedyaManager has no language MENU or LIST
// anywhere in its UI yet to build it for (Part B applies to "every list or
// menu a person chooses a language from" — there is none here). The rule
// engine's `<Language>` template output and its rule conditions, added
// below, are closer to "canonical" than "presentation" in spirit (they
// read and compare a STORED value, not present a menu), so they are built
// here rather than waiting on a presentation profile that has nothing to
// attach to yet. So this module needs LANG-001 through LANG-003 and
// TRACK-070 — canonicalising a value, reading an old three-letter code,
// and writing the right form per tag container. It does not need
// ordering, matching, or automatic selection; those live entirely inside
// `meedya_lang` already and MeedyaManager has nothing to call them with
// yet.
//
// Two different situations, kept apart on purpose:
//
//   * READING a value already sitting in a file (`parse_stored_language`)
//     — this can be an old file nobody can go back and ask about, so it
//     NEVER refuses: an unrecognised value becomes `und` ("not known",
//     LANG-003), and the original text is kept alongside so nothing is
//     lost and a person can fix it (COMPAT-040).
//   * A PERSON typing a language on purpose — the CLI's `--set
//     language=...`, a future editor, a future rule — is a different
//     situation, and `parse_language_input` REFUSES rather than guessing,
//     because silently writing `und` for a typing mistake would hide the
//     mistake from the very person who could fix it on the spot.
//
// Both readers accept the same shapes (a two-letter BCP 47 tag, a full
// tag such as `pt-BR`, or an old three-letter code such as `fre`), because
// both go through `meedya_lang::from_legacy_three_letter` — the LANG-002
// reader, which is written to accept a value already known to be a BCP 47
// tag just as happily as an old three-letter code.
//
// One LANG-002 case this module does NOT implement any splitting code
// for, but for a narrower reason than first thought — corrected after an
// independent review actually built and ran the case rather than trusting
// the claim below at face value: "ID3v2.4 TLAN can hold several codes
// separated by a null character; split them first, the first is the
// primary." Writing a genuine null-separated TLAN into a real MP3 and
// reading it straight back with `lofty` shows the null survives to disk,
// and `lofty`'s own ID3v2 reader splits it into SEPARATE items — the same
// multi-value handling every other ID3 text frame already gets — before
// this crate ever sees a raw string, so `metadata::extract_tags` right
// after such a write already returns a plain `vec!["eng", "swe"]`, with no
// null character anywhere in it. So far this still holds, and LANG-002's
// "the first is the primary language" is still, for that IMMEDIATE case,
// simply "take the first element of that vector".
//
// What does NOT hold — found by the review, reproduced here, and now
// covered by an ignored regression test rather than silently left wrong —
// is that this survives a LATER, UNRELATED save. Once `write_tags` is
// called again for some other field entirely (changing only the title,
// say), only the SECOND of the two values survives; the first is gone
// permanently. This is not specific to `language` at all — the exact same
// loss happens to a multi-value `artist` field — so it is a general fault
// in how this crate currently round-trips ANY multi-value ID3 field
// through a save, not a language-policy gap, and it is tracked and fixed
// on its own timescale as issue #254 rather than folded into this work.
// See `multi_value_tlan_does_not_survive_an_unrelated_save` (marked
// `#[ignore = "issue #254"]`) in
// `crates/mm-core/tests/metadata_roundtrip.rs`.

use std::path::Path;

use lofty::tag::TagType;

use meedya_lang::{
    Iso639Form, LanguageTag, TagNote, canonicalise, from_legacy_three_letter, iso639_2_code,
};

// ---------------------------------------------------------------------------
// Reading a language value that is already sitting in a file
// ---------------------------------------------------------------------------

/// A language value read out of a file, kept two ways at once (COMPAT-040):
/// what it structurally means, and the exact text the file held.
///
/// Editors and templates SHOULD keep showing [`raw`](Self::raw) — "the
/// editors show raw key/value today", and this module does not change
/// that — and reach for [`tag`](Self::tag) only where a genuinely
/// structured language is needed (there is no such caller in
/// MeedyaManager yet; this type exists so the day one is added, it reads
/// the value the one correct way rather than re-inventing LANG-002).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredLanguage {
    /// What LANG-002's reader made of the value. `und` ("language not
    /// known") both when the value genuinely could not be recognised
    /// (LANG-003 — never a guess) AND when it was recognised as ID3's own
    /// `XXX` "not known" marker — see [`recognised`](Self::recognised) for
    /// how to tell those two situations apart.
    pub tag: LanguageTag,
    /// The value exactly as it was passed in — the ENTIRE input,
    /// untouched, not even the four whitespace characters LANG-001 step 1
    /// would trim. (An earlier version of this doc comment said this field
    /// was trimmed; it never was. Kept fully untouched, on purpose: this
    /// is the "original text" COMPAT-040 wants shown to a person, and the
    /// original is more useful than a partially-processed version of it.)
    pub raw: String,
    /// `true` when LANG-002's reader could make sense of `raw` at all
    /// (including ID3's `XXX` marker, which it turns into `und` on
    /// purpose). `false` means `tag` is `und` only because nothing above
    /// recognised `raw` — this is the field to check before deciding
    /// whether to show `tag.tag` or `raw` to a person: showing `tag.tag`
    /// for a recognised value is fine (`XXX` → "und" is the correct,
    /// intended standard form), but showing it for an UNRECOGNISED one
    /// would silently replace a person's own words with a guess-shaped
    /// placeholder, which is exactly what LANG-003 forbids.
    pub recognised: bool,
}

/// Read a raw `language` tag value the way LANG-002 requires.
///
/// Accepts an old three-letter code, a Matroska-style "three letters +
/// region", or an already-canonical BCP 47 tag, in that order — never as a
/// guess. This function cannot fail. A value nothing above recognises becomes
/// `und` (LANG-003), with `raw` carrying the original text so it is never
/// silently lost (COMPAT-040) and a person can still fix it.
pub fn parse_stored_language(raw: &str) -> StoredLanguage {
    match from_legacy_three_letter(raw) {
        Some(tag) => StoredLanguage {
            tag,
            raw: raw.to_string(),
            recognised: true,
        },
        None => StoredLanguage {
            tag: canonicalise("und"),
            raw: raw.to_string(),
            recognised: false,
        },
    }
}

/// The form a `language` value should be RENDERED or COMPARED in.
///
/// Used wherever the value means the same thing regardless of which tag
/// container a particular file happens to use — the rule engine's
/// `<Language>` template output, and both sides of a `language` rule
/// condition, use this. Without it, an
/// MP3 storing the old three-letter code `eng` and a FLAC storing the
/// short code `en` are the SAME fact (English) but compare and render as
/// DIFFERENT text, so a rule written and tested against one format can
/// silently stop matching files in another the moment this crate starts
/// writing the correct per-format form (see the `write_tags` doc comment
/// for TRACK-070) — found, and reproduced with a rule that matched MP3s
/// before this crate wrote `en` there and stopped matching them once it
/// correctly started writing `eng`, while this rule was being reviewed.
///
/// Returns the standard tag ([`parse_stored_language`]'s `tag.tag`) when
/// the value is recognised, and the ORIGINAL TEXT UNCHANGED when it is
/// not — never a guess (LANG-003), and never `und` standing in for
/// something a person actually typed that this crate simply could not
/// parse.
pub fn standardise_for_comparison(raw: &str) -> String {
    let stored = parse_stored_language(raw);
    if stored.recognised {
        stored.tag.tag
    } else {
        stored.raw
    }
}

// ---------------------------------------------------------------------------
// A person setting or changing a language on purpose
// ---------------------------------------------------------------------------

/// A language value a person typed could not be understood at all.
///
/// Carries the input back, both so the message can quote it and so a
/// caller further up (the CLI, a future editor) does not have to keep hold
/// of it separately just to report the failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputError {
    pub input: String,
}

impl std::fmt::Display for LanguageInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Plain English, no jargon (house rule): "BCP 47" is a standard's
        // name, not something a person setting a language needs to know —
        // say what a valid answer looks like instead. This message itself
        // never names a temporary file or anything internal to how this
        // crate works (item 13 of the language-policy review): a refusal
        // is about what the PERSON typed, not about MeedyaManager's own
        // plumbing.
        //
        // That guarantee held here but not end to end (found by the
        // second review round, item 7): `integrity::mutate_file_safe`
        // used to wrap THIS message in `"mutation failed on '{target}':
        // ..."`, where `target` is Test Mode's own `_MeedyaManager` copy
        // path — so the text this function builds was still clean, but
        // what an app actually showed on screen was not, by the time it
        // had passed through the write guard. Fixed in `integrity.rs`,
        // not here; this comment is corrected so it no longer claims a
        // guarantee this one function cannot make on its own.
        write!(
            f,
            "'{}' is not a language MeedyaManager recognises. Use a language code such as \
             \"en\", \"pt-BR\" or \"zh-Hant\" (an older three-letter code such as \"fre\" is \
             accepted too), or \"und\" if the language is genuinely not known.",
            self.input
        )
    }
}

impl std::error::Error for LanguageInputError {}

/// Read a language value a PERSON chose on purpose.
///
/// Used at the CLI's `--set language=...`, a future metadata editor, or a
/// future rule action — the same LANG-002 reader [`parse_stored_language`]
/// uses, so `en-GB` and `eng` are both accepted equally.
///
/// Unlike [`parse_stored_language`], this REFUSES a value nothing
/// recognises rather than falling back to `und`. Reading an old file that
/// nobody can go back and ask about is one situation; a person typing a
/// language right now, with a chance to be told they made a mistake, is a
/// different one — and silently accepting the mistake as `und` would hide
/// it from the one person who could still fix it.
///
/// # Errors
/// Returns [`LanguageInputError`] naming the input, with an example of what
/// a valid one looks like, when nothing above recognises it.
pub fn parse_language_input(input: &str) -> Result<LanguageTag, LanguageInputError> {
    from_legacy_three_letter(input).ok_or_else(|| LanguageInputError {
        input: input.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Writing a language value into a specific tag container (TRACK-070)
// ---------------------------------------------------------------------------

/// What TRACK-070 says to write into the `language` tag's slot for one
/// specific container format.
///
/// Every format this crate writes maps the MeedyaManager `language` key
/// onto lofty's `ItemKey::Language`, but TRACK-070's table does not treat
/// every container the same way once it gets there:
///
/// * **ID3 (`TLAN`)** — ID3 has no field for a full BCP 47 tag at all, so
///   TRACK-070 says to write the ISO 639-2 **terminology** three-letter
///   code (`deu`, not the bibliographic `ger`) — this is the one format
///   here where the slot cannot hold what was actually asked for, only an
///   approximation of it.
/// * **Vorbis `LANGUAGE`, the MP4 freeform `LANGUAGE` item, APE
///   `Language`, and RIFF `ILNG`** — each of these is a single free-text
///   field with no separate "full tag" slot to split the work between
///   (unlike ID3, which at least has nothing else trying to hold more),
///   so TRACK-070 has them carry the canonical tag itself, exactly as
///   Vorbis and the MP4 freeform item are named explicitly in its table.
///
/// Returns the ISO 639-2 terminology code for [`TagType::Id3v2`], and the
/// canonical tag string for every other container lofty can write this
/// crate's `language` key into (`Ape`, `Mp4Ilst`, `VorbisComments`,
/// `RiffInfo`).
///
/// A note on `RiffInfo` in particular, found while testing this against a
/// real WAV file rather than assumed from the policy table: lofty's own
/// `FileType::primary_tag_type()` maps `FileType::Wav` to `TagType::Id3v2`,
/// the same as MP3 — a WAV file's PRIMARY tag is therefore always an
/// embedded ID3v2 one, never `RiffInfo`, however the file's own metadata
/// module doc comment describes it. This function still handles
/// `TagType::RiffInfo` correctly (the canonical tag, matching the policy).
///
/// **Corrected after the second language-policy review round**: this used
/// to say `write_tags` could never actually reach `RiffInfo` at all, "for
/// whichever caller does reach it" being read as some hypothetical future
/// caller. That stopped being true the moment `write_tags` gained its
/// "Item 5" fix (keeping every tag container a file already has a
/// language value in consistent, not only the primary one): a WAV that
/// already carries a genuine `RiffInfo` language value — from some other
/// tool, or from an earlier write by this very function — has THAT
/// container updated too, through this exact function, every time
/// `language` is set. See `setting_language_keeps_every_tag_container_consistent`
/// in `crates/mm-core/tests/metadata_roundtrip.rs`. What is still true,
/// and is the actual reason a FRESH WAV never gets a `RiffInfo` tag from
/// nothing: `write_tags` only ever updates a container that already has a
/// language value (see `language_write_targets`), and a fresh WAV's
/// primary — the only container it starts with — is ID3v2, not `RiffInfo`.
/// See `wav_write_tags_uses_embedded_id3v2_not_riff_info` in
/// `crates/mm-core/tests/metadata_roundtrip.rs` for the test that found
/// the underlying primary-tag-type fact, and the note in this crate's
/// write-up of this work for why fixing that more general (not
/// language-specific) fact is out of scope here.
pub fn language_value_for_tag_type(tag: &LanguageTag, tag_type: TagType) -> String {
    match tag_type {
        TagType::Id3v2 => iso639_2_code(tag, Iso639Form::Terminology),
        _ => tag.tag.clone(),
    }
}

// ---------------------------------------------------------------------------
// Telling a person when what gets stored differs from what they typed
// ---------------------------------------------------------------------------

/// If writing `input` as `language` into a container of type `tag_type`
/// would lose or reshape something a person actually typed — TRACK-070's
/// per-format conversion can genuinely lose information, most concretely
/// an ID3 file's region and script — this explains what would actually be
/// stored, and why, in plain words. Returns `None` when `input` cannot be
/// understood at all (the caller's own refusal message already covers
/// that case, via [`parse_language_input`]) or when there is nothing worth
/// telling a person about.
///
/// Deliberately NOT triggered merely by `stored != input` as text: turning
/// `en` into ID3's `eng`, or `EN-gb` into the tidied-up `en-GB`, changes
/// the letters on screen but loses nothing and would make this fire on
/// every ordinary edit — noise, not the genuine "your region just vanished"
/// warning this exists for. A note only appears when there is an actual
/// REASON to give: ID3 dropping a region/script/variant or having no
/// three-letter code at all, or one of the crate's own notes about the tag
/// (an unregistered subtag, one with no single replacement, or one the
/// registry replaced outright).
///
/// Item 6 of the first language-policy review round: a person setting
/// `language=pt-BR` on an MP3 sees no error (the value IS a real language)
/// and no obvious sign that only `por` — the whole region silently gone —
/// was actually written, unless something tells them so.
///
/// Redesigned for review item 4 of the SECOND round, which found this
/// describing a PLAN rather than what [`super::write_tags`] would really
/// do: it used to take one `tag_type` (the primary container only) and
/// say "an MP3", even though the exact same ID3v2 tag type can be embedded
/// inside a WAV file too (see [`language_value_for_tag_type`]'s own doc
/// comment on that surprise) and a file can genuinely need MORE than one
/// container updated at once (see `write_tags`'s "Item 5" doc comment).
/// `tag_types` is now every container [`language_write_targets`] says
/// `write_tags` will actually touch, so this function's answer can never
/// name a container the real write does not reach, and never miss one it
/// does.
fn describe_conversion_for_types(input: &str, tag_types: &[TagType]) -> Option<String> {
    let parsed = parse_language_input(input).ok()?;

    let mut reasons: Vec<String> = Vec::new();

    // The structural TRACK-070 reason only an ID3 tag's three-letter-only
    // field can run into — every other container this crate writes into
    // keeps the canonical tag whole, so nothing is ever lost writing into
    // one of those (a text difference there, if any, is pure
    // normalisation, covered by the notes below rather than here). Named
    // "an ID3 tag", never "an MP3" (review item 4): the same tag type is
    // reached from a `.wav` file just as often as from a `.mp3` one.
    if tag_types.contains(&TagType::Id3v2) {
        let stored = language_value_for_tag_type(&parsed, TagType::Id3v2);
        if stored == "und" {
            // Review item 9: say "not known" — the plain-English fact —
            // rather than showing the raw three-letter code "und" (or, on
            // the READING side, ID3's own "xxx" marker) as if it meant
            // something to a reader who has never heard of either.
            reasons.push(format!(
                "an ID3 tag has no three-letter code for \"{}\" at all, so it will be stored \
                 there as not known",
                parsed.language.as_deref().unwrap_or(&parsed.tag)
            ));
        } else {
            // Review item 9: name exactly which part(s) are lost, rather
            // than always listing "the region, script or extra detail"
            // whether or not the input actually had all three.
            let mut dropped: Vec<&str> = Vec::new();
            if parsed.region.is_some() {
                dropped.push("region");
            }
            if parsed.script.is_some() {
                dropped.push("script");
            }
            if !parsed.variants.is_empty() {
                dropped.push("extra detail");
            }
            if !parsed.extensions.is_empty() {
                dropped.push("extension");
            }
            if !parsed.private_use.is_empty() {
                dropped.push("private-use part");
            }
            if !dropped.is_empty() {
                reasons.push(format!(
                    "an ID3 tag can only hold the three-letter language code, so it will lose \
                     the {} you typed — it will be stored there as \"{stored}\"",
                    join_with_and(&dropped)
                ));
            }
        }
    }

    // The crate's own notes about the tag itself — an unregistered subtag,
    // a deprecated one with no single replacement, or a subtag the
    // registry replaced outright — explain why the STANDARD form is not
    // simply `input` restated. Genuine case folding or whitespace trimming
    // alone never produces one of these, so it never reaches this point at
    // all — exactly the "no noise for an ordinary edit" property this
    // function exists to have.
    let mut note_reasons: Vec<String> = Vec::new();
    for note in &parsed.notes {
        match note {
            TagNote::UnregisteredSubtag { subtag } => note_reasons.push(format!(
                "\"{subtag}\" is not on the official list of language subtags, but it is kept \
                 exactly as typed"
            )),
            TagNote::DeprecatedNoReplacement { subtag } => note_reasons.push(format!(
                "\"{subtag}\" is an old code with no single replacement, so it is kept exactly \
                 as typed"
            )),
            TagNote::SubtagReplaced { from, to } => {
                note_reasons.push(format!(
                    "\"{from}\" is written as \"{to}\" in the standard form"
                ));
            }
        }
    }

    // Review item 9: a WHOLE tag being replaced — a grandfathered tag such
    // as "i-klingon" becoming "tlh", or a redundant combination such as
    // "sgn-BR" becoming "bzs" — carries NO note of its own in
    // `parsed.notes`: the shared crate's `canonicalise` recurses straight
    // into canonicalising the replacement and returns THAT result, with
    // no record left behind that the input was ever anything else (unlike
    // a single-subtag replacement such as "iw" -> "he", which the
    // `SubtagReplaced` loop above already reports). Detected here by
    // comparing the input, case-folded, against the tag `parsed` actually
    // is — only when nothing above already explains the difference, so a
    // case already covered (like "iw") is never reported twice.
    if note_reasons.is_empty() && !parsed.tag.eq_ignore_ascii_case(input.trim()) {
        note_reasons.push(format!(
            "\"{}\" is an old or grouped form that is no longer used — it is replaced with the \
             current code, \"{}\"",
            input.trim(),
            parsed.tag
        ));
    }

    reasons.extend(note_reasons);

    if reasons.is_empty() {
        // Nothing worth reporting — either `stored == input` outright, or
        // the only difference is ordinary normalisation (case, the plain
        // 2-to-3-letter ID3 form with no region/script/variant to lose).
        return None;
    }

    Some(reasons.join("; "))
}

/// Joins a short list of plain-English part names the way a person would
/// say them out loud: `"region"`, `"region and script"`, or `"region,
/// script and extra detail"` — used only to name which specific part(s) of
/// a language tag an ID3 tag's three-letter-only field cannot hold
/// (review item 9 of the second language-policy review round: the message
/// used to say "the region, script or extra detail" regardless of which,
/// if any, of the three the input actually had).
fn join_with_and(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [one] => (*one).to_string(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let (last, rest) = items.split_last().expect("checked non-empty above");
            format!("{} and {last}", rest.join(", "))
        }
    }
}

/// The same explanation as this module's private `describe_conversion_for_types`, for a file
/// on disk.
///
/// Reads the file (never writes to it) purely to find out (a) whether
/// [`super::write_tags`] would treat `input` as a genuine change at all —
/// COMPAT-030, the same comparison `write_tags` itself makes, via the
/// same shared (but not public — hence plain text, not a doc link)
/// `current_joined_value` — and (b) which containers it would actually
/// touch if so, via the same shared `language_write_targets`. Both checks
/// are done at Phase-1 validation time, before any write happens, so the
/// CLI can show this note even on
/// `--dry-run` (which never calls `write_tags` at all) — and, since review
/// item 4 of the second review round, so the note can never say something
/// `write_tags` would not really do: reusing the exact functions
/// `write_tags` itself uses is what makes that a guarantee rather than a
/// hope. Returns `None` when the file cannot even be probed, when `input`
/// is identical to what is already stored (nothing will change, so there
/// is nothing to say — review item 4's other finding), or when there is
/// nothing worth telling a person about. A caller that goes on to actually
/// write will get a real, specific error for a value nothing recognises at
/// all, from `write_tags` itself.
pub fn preview_conversion_note(path: &Path, input: &str) -> Option<String> {
    let tagged_file = super::open_tagged_file(path).ok()?;
    if super::current_joined_value(&tagged_file, super::TAG_LANGUAGE).as_deref() == Some(input) {
        return None;
    }
    let tag_types = super::language_write_targets(&tagged_file);
    describe_conversion_for_types(input, &tag_types)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_conversion_reports_a_lost_region_on_id3() {
        let note =
            describe_conversion_for_types("pt-BR", &[TagType::Id3v2]).expect("a region is lost");
        assert!(
            note.contains("\"por\""),
            "must show the stored form: {note:?}"
        );
        assert!(
            note.to_lowercase().contains("region"),
            "must say a region was lost: {note:?}"
        );
        assert!(
            note.contains("an ID3 tag"),
            "must name the tag format, not assume the file is an MP3: {note:?}"
        );
    }

    #[test]
    fn describe_conversion_reports_a_language_with_no_639_2_code() {
        // "yue" (Cantonese) is a genuine subtag with no ISO 639-2 code at
        // all, so an ID3 tag can only record it as "not known" — review
        // item 9 of the second review round: say "not known" in plain
        // English, not the raw three-letter code "und".
        let note =
            describe_conversion_for_types("yue", &[TagType::Id3v2]).expect("no 639-2 code exists");
        assert!(
            note.to_lowercase().contains("not known"),
            "must say \"not known\", not the raw code: {note:?}"
        );
        assert!(
            !note.contains("\"und\""),
            "must not show the raw code: {note:?}"
        );
        assert!(note.contains("\"yue\""));
    }

    /// Copy-update sweep to core `aaaa585`: two-letter subtags in the `qb`
    /// to `qt` range sit alphabetically between the genuine three-letter
    /// local-use codes `qaa` and `qtz`, but are not local-use codes
    /// themselves — the crate now writes `und` for them and notes the
    /// subtag as unregistered (previously this range was not specifically
    /// handled). This is the same shape as `yue` above (no ISO 639-2 code
    /// at all), but is worth its own test: it is a NEW behaviour from the
    /// crate update, not a pre-existing one, and it is reached through the
    /// `parse_language_input` -> `describe_conversion_for_types` path a
    /// person actually types into, not just the shared crate's own unit
    /// tests.
    #[test]
    fn describe_conversion_reports_a_two_letter_q_code_as_und_on_id3() {
        let note =
            describe_conversion_for_types("qb", &[TagType::Id3v2]).expect("qb has no 639-2 code");
        assert!(note.to_lowercase().contains("not known"), "{note}");
        assert!(note.contains("\"qb\""), "{note}");
    }

    /// The same input on a container that keeps the canonical tag whole
    /// (no ID3 three-letter restriction) still gets a note — not because
    /// anything was LOST, but because the crate flags "qb" as a subtag
    /// nothing recognises, and `describe_conversion_for_types` surfaces
    /// every note the crate itself records, not only the ID3-specific
    /// ones.
    #[test]
    fn describe_conversion_reports_a_two_letter_q_code_as_unregistered_everywhere() {
        let note = describe_conversion_for_types("qb", &[TagType::VorbisComments])
            .expect("qb is unregistered");
        assert!(note.contains("\"qb\""), "{note}");
        assert!(note.contains("not on the official list"), "{note}");
    }

    #[test]
    fn describe_conversion_reports_a_registry_replacement() {
        let note = describe_conversion_for_types("iw", &[TagType::VorbisComments])
            .expect("iw is replaced");
        assert!(note.contains("\"iw\"") && note.contains("\"he\""));
    }

    /// Review item 9 of the second review round: replacing a WHOLE tag —
    /// a grandfathered one with a single-tag preferred replacement, or a
    /// redundant combination the registry collapses to one tag — carries
    /// no `TagNote` of its own (unlike a single-subtag replacement such as
    /// "iw" -> "he", the test above), because the shared crate's
    /// `canonicalise` recurses straight into the replacement and returns
    /// it with no memory of the original input. Before this fix, setting
    /// either of these produced NO note at all — a person typing
    /// "i-klingon" would see their input silently become "tlh" with
    /// nothing telling them so.
    #[test]
    fn describe_conversion_reports_a_replaced_grandfathered_tag() {
        let note = describe_conversion_for_types("i-klingon", &[TagType::VorbisComments])
            .expect("i-klingon is a grandfathered tag with a single-tag replacement");
        assert!(note.contains("\"i-klingon\""), "{note}");
        assert!(note.contains("\"tlh\""), "{note}");
    }

    /// The redundant-tag case: "sgn-BR" (a specific, well-formed sign-
    /// language-plus-region combination) has a single registered
    /// replacement, "bzs", and — like the grandfathered case above — the
    /// crate's own recursion into `canonicalise("bzs")` leaves no note
    /// behind explaining why the two look nothing alike.
    #[test]
    fn describe_conversion_reports_a_replaced_redundant_tag() {
        let note = describe_conversion_for_types("sgn-BR", &[TagType::VorbisComments])
            .expect("sgn-BR is a redundant tag with a single-tag replacement");
        assert!(note.contains("\"sgn-BR\""), "{note}");
        assert!(note.contains("\"bzs\""), "{note}");
    }

    #[test]
    fn describe_conversion_is_silent_for_ordinary_lossless_conversions() {
        // The plain two-to-three-letter ID3 form, with no region, script or
        // variant to lose, is not a loss — nothing is silently dropped.
        assert_eq!(describe_conversion_for_types("en", &[TagType::Id3v2]), None);
        // Pure case-folding / whitespace tidying on a container that keeps
        // the canonical tag whole is not a loss either.
        assert_eq!(
            describe_conversion_for_types("EN-gb", &[TagType::VorbisComments]),
            None
        );
        assert_eq!(
            describe_conversion_for_types("pt-BR", &[TagType::VorbisComments]),
            None
        );
    }

    #[test]
    fn describe_conversion_returns_none_for_input_it_cannot_parse_at_all() {
        // Not this function's job — the caller's own refusal (from
        // parse_language_input) already covers a value that makes no
        // sense as a language at all.
        assert_eq!(
            describe_conversion_for_types("not a language", &[TagType::Id3v2]),
            None
        );
    }

    /// Review item 4 of the second review round: a file can need more than
    /// one container updated at once (a WAV with both a RIFF INFO value
    /// and an embedded ID3v2 tag already carrying one). The ID3 loss
    /// reason must still be named specifically for ITS container even
    /// when another container in the same list keeps the value whole —
    /// the note is about the ID3 tag, not a claim that the region is lost
    /// everywhere.
    #[test]
    fn describe_conversion_names_the_id3_loss_even_alongside_a_full_container() {
        let note = describe_conversion_for_types("pt-BR", &[TagType::RiffInfo, TagType::Id3v2])
            .expect("the ID3 half of this write still loses the region");
        assert!(note.contains("an ID3 tag"), "{note}");
        assert!(note.contains("\"por\""), "{note}");
    }

    /// The companion case: when NEITHER container in the list is an ID3
    /// tag, nothing is lost structurally — this must stay silent exactly
    /// as the single-container version does.
    #[test]
    fn describe_conversion_is_silent_when_no_target_is_id3() {
        assert_eq!(
            describe_conversion_for_types("pt-BR", &[TagType::RiffInfo, TagType::VorbisComments]),
            None
        );
    }

    #[test]
    fn parse_stored_language_accepts_a_legacy_three_letter_code() {
        let stored = parse_stored_language("fre");
        assert_eq!(stored.tag.tag, "fr");
        assert_eq!(stored.raw, "fre");
    }

    #[test]
    fn parse_stored_language_accepts_a_full_bcp47_tag_already() {
        let stored = parse_stored_language("pt-BR");
        assert_eq!(stored.tag.tag, "pt-BR");
        assert_eq!(stored.raw, "pt-BR");
    }

    #[test]
    fn parse_stored_language_reads_only_the_primary_of_a_multi_value_tlan() {
        // ID3v2.4's TLAN frame can hold several three-letter codes separated
        // by a null character; LANG-002 says the first is the primary
        // language, and the rest are not read at all. `\u{0}` here is a
        // genuine null byte, the same shape a real TLAN frame with two
        // values ("eng" and "swe") would contain, not an escaped string.
        let stored = parse_stored_language("eng\u{0}swe");
        assert_eq!(stored.tag.tag, "en");
    }

    #[test]
    fn parse_stored_language_never_fails_and_never_guesses() {
        // "zzz" is not a registered code of any kind (LANG-002 step 5): the
        // structured value is `und`, per LANG-003, and the original text is
        // kept so a person can still fix it (COMPAT-040) — never dropped,
        // never turned into a guess.
        let stored = parse_stored_language("zzz");
        assert_eq!(stored.tag.tag, "und");
        assert_eq!(stored.raw, "zzz");
    }

    #[test]
    fn parse_stored_language_keeps_id3s_own_not_known_marker_as_und() {
        let stored = parse_stored_language("XXX");
        assert_eq!(stored.tag.tag, "und");
        assert_eq!(stored.raw, "XXX");
    }

    #[test]
    fn parse_language_input_accepts_both_shapes() {
        assert_eq!(parse_language_input("en-GB").unwrap().tag, "en-GB");
        assert_eq!(parse_language_input("fre").unwrap().tag, "fr");
        assert_eq!(parse_language_input("und").unwrap().tag, "und");
    }

    #[test]
    fn parse_language_input_refuses_a_language_name_rather_than_a_tag() {
        // "French" is a name, not a tag (LANG-001 step 3: a primary language
        // of 4+ letters is only accepted when the registry lists it, and no
        // language name is ever registered as a subtag).
        let err = parse_language_input("French").unwrap_err();
        assert_eq!(err.input, "French");
        // The message must give the person a working example, not just say
        // no. Checked against the exact example text (`"pt-BR"`), not the
        // bare letters "en" — a message that dropped every example but
        // still happened to print the word "English" or the input "French"
        // would wrongly pass a looser check (item 8 of the language-policy
        // review; proven by deleting the examples and confirming this
        // assertion is what catches it, not a coincidence of the input).
        let message = err.to_string();
        assert!(
            message.contains("\"pt-BR\""),
            "refusal message must show the working example \"pt-BR\", got: {message:?}"
        );
        assert!(
            !message.to_lowercase().contains("bcp"),
            "refusal message must not use the jargon term \"BCP 47\", got: {message:?}"
        );
    }

    #[test]
    fn parse_language_input_refuses_gibberish() {
        assert!(parse_language_input("zzz").is_err());
        assert!(parse_language_input("not a language").is_err());
        assert!(parse_language_input("").is_err());
    }

    #[test]
    fn language_value_for_tag_type_writes_terminology_form_for_id3() {
        // German: bibliographic "ger" vs terminology "deu" — the two forms
        // genuinely differ (TRACK-070's "differ for twenty languages"), so
        // this is a real check, not a coincidence of a language where both
        // forms happen to match.
        let de = canonicalise("de");
        assert_eq!(language_value_for_tag_type(&de, TagType::Id3v2), "deu");
    }

    #[test]
    fn language_value_for_tag_type_writes_the_canonical_tag_everywhere_else() {
        let pt_br = canonicalise("pt-BR");
        for tag_type in [
            TagType::VorbisComments,
            TagType::Mp4Ilst,
            TagType::Ape,
            TagType::RiffInfo,
        ] {
            assert_eq!(language_value_for_tag_type(&pt_br, tag_type), "pt-BR");
        }
    }

    #[test]
    fn language_value_for_tag_type_writes_und_to_id3_for_a_language_with_no_639_2_code() {
        // Cantonese ("yue") is a genuine BCP 47 / ISO 639-3 subtag with no
        // ISO 639-2 code at all — TRACK-070 says `und` here, never a guess.
        let yue = canonicalise("yue");
        assert_eq!(language_value_for_tag_type(&yue, TagType::Id3v2), "und");
        // Everywhere else it is written as itself: nothing was lost by not
        // having a three-letter form, because those slots hold the real tag.
        assert_eq!(
            language_value_for_tag_type(&yue, TagType::VorbisComments),
            "yue"
        );
    }
}
