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
// MeedyaManager's profile today is "canonical" only (policy section 2):
// it edits stored metadata, and has no language menus of its own yet (the
// "presentation" profile would add those). So this module only needs
// LANG-001 through LANG-003 and TRACK-070 — canonicalising a value,
// reading an old three-letter code, and writing the right form per tag
// container. It does not need ordering, matching, or automatic selection;
// those live entirely inside `meedya_lang` already and MeedyaManager has
// nothing to call them with yet.
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
// One LANG-002 case this module does NOT implement, on purpose, because
// testing it against a real file showed it was already handled: "ID3v2.4
// TLAN can hold several codes separated by a null character; split them
// first, the first is the primary." Writing a genuine null-separated TLAN
// into a real MP3 and reading it back with `lofty` shows the null survives
// to disk, but `lofty`'s own ID3v2 reader splits it into SEPARATE items —
// the same multi-value handling every other ID3 text frame already gets —
// before this crate ever sees a raw string. `metadata::extract_tags`
// therefore already returns a plain `vec!["eng", "swe"]` for such a file,
// with no null character anywhere in it, through the generic per-item loop
// every tag key shares. LANG-002's "the first is the primary language"
// becomes, for MeedyaManager, simply: take the first element of that
// vector. See `multi_value_tlan_is_split_into_separate_items_by_lofty_itself`
// in `crates/mm-core/tests/metadata_roundtrip.rs` for the test that found
// this (the raw file bytes were inspected directly to confirm the null
// really does reach disk, ruling out "lofty silently drops it on write" as
// the explanation).

use lofty::tag::TagType;

use meedya_lang::{Iso639Form, LanguageTag, canonicalise, from_legacy_three_letter, iso639_2_code};

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
    /// known") when the value could not be recognised at all — never a
    /// guess (LANG-003).
    pub tag: LanguageTag,
    /// The value exactly as read from the file, after only the trimming
    /// LANG-002 itself does (trailing null padding, and the four
    /// whitespace characters LANG-001 step 1 names). Never altered further
    /// — this is what "keep the original text" means in practice.
    pub raw: String,
}

/// Read a raw `language` tag value the way LANG-002 requires.
///
/// Accepts an old three-letter code, a Matroska-style "three letters +
/// region", or an already-canonical BCP 47 tag, in that order — never as a
/// guess. This function cannot fail. A value nothing above recognises becomes
/// `und` (LANG-003), with `raw` carrying the original text so it is never
/// silently lost (COMPAT-040) and a person can still fix it.
pub fn parse_stored_language(raw: &str) -> StoredLanguage {
    let tag = from_legacy_three_letter(raw).unwrap_or_else(|| canonicalise("und"));
    StoredLanguage {
        tag,
        raw: raw.to_string(),
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
        write!(
            f,
            "'{}' is not a language MeedyaManager recognises. Use a BCP 47 language tag \
             such as \"en\", \"pt-BR\" or \"zh-Hant\" (an old three-letter code such as \
             \"fre\" is accepted too), or \"und\" if the language is genuinely not known.",
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
/// the same as MP3 — a WAV file therefore gets an embedded ID3v2 tag from
/// `write_tags`, never a `RiffInfo` one, however the file's own metadata
/// module doc comment describes it. This function still handles
/// `TagType::RiffInfo` correctly (the canonical tag, matching the policy),
/// for whichever caller does reach it — reading a file some other tool
/// already gave a genuine RIFF INFO tag, say — but `write_tags` itself
/// cannot produce that path today. See `wav_riff_info_round_trip` in
/// `crates/mm-core/tests/metadata_roundtrip.rs` for the test that found
/// this, and the note in this crate's write-up of this work for why fixing
/// that more general (not language-specific) fact is out of scope here.
pub fn language_value_for_tag_type(tag: &LanguageTag, tag_type: TagType) -> String {
    match tag_type {
        TagType::Id3v2 => iso639_2_code(tag, Iso639Form::Terminology),
        _ => tag.tag.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // The message must give the person a way forward, not just say no.
        assert!(err.to_string().contains("en"));
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
