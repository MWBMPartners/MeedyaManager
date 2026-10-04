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

/// Split ONE stored language field into its separate values, in order.
///
/// LANG-002: "If the field holds several values (ID3v2.4 separates them
/// with a null character), split them first and read each on its own; the
/// first is the primary language." Codex's catch-up review of the
/// language-policy branch, finding 4: this used to happen for ID3 only, and
/// only because the `lofty` tag library's ID3 reader splits a field on its
/// zero characters before this crate sees it. APE keeps several values in
/// ONE item the same way (zero characters between them) and `lofty` hands
/// that over whole, as can a Vorbis comment, the MP4 freeform item or a
/// RIFF INFO entry written by another tool. Reproduced with the code as it
/// was at `a150926` on a real MP3 with an APE `Language` item of `eng`,
/// zero, `fra`: it was read as ONE value, `"eng\u{0}fra"`, so a rule's
/// `language Matches "fra"` matched while a path was being built from the
/// first language (English), and display mode showed only `en` — French
/// lost.
///
/// So every stored value goes through here, whatever format it came from,
/// and comes out exactly as the ID3 path already gave it: split at every
/// zero character, each part trimmed of LANG-001 step 1's four whitespace
/// characters only (space, tab, line feed, carriage return — see
/// `trim_lang_whitespace`), and empty parts left out (which is also what happens to the empty parts
/// `lofty` produces for an ID3 field with a zero at its end, or two zeros
/// in a row). Order is kept, so the first is still the primary language.
/// A value with no zero character comes back as itself, trimmed (or not at
/// all, when only whitespace).
///
/// What this cannot do: tell a value that is genuinely several languages
/// from one some tool padded oddly — a zero always separates, as LANG-002
/// says. Callers that need "one value" ([`standardise_for_comparison`],
/// [`parse_stored_language`]) are given the parts one at a time.
///
/// Codex's catch-up review, finding 5: the trim used to be Rust's
/// `str::trim`, which also removes a no-break space and every other Unicode
/// space. The policy says "those four characters and no others. Anything
/// else, a no-break space included, is part of the value and makes it
/// malformed." Reproduced with the code as it was at `a150926` on a real
/// FLAC whose `LANGUAGE` was a no-break space then `en`: it was read as
/// plain `en`, `<Language>` gave `en`, and `language Equals en` matched. The
/// value now keeps its no-break space, so it is not recognised, is shown as
/// stored, and matches nothing but itself.
pub fn split_stored_values(raw: &str) -> Vec<String> {
    raw.split('\u{0}')
        .map(trim_lang_whitespace)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
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
///
/// Takes ONE value. A stored field that may hold several, separated by a
/// zero character, must be split with [`split_stored_values`] first —
/// every caller in this crate does (Codex's catch-up review, finding 4:
/// given several at once, the shared reader answers for the first alone,
/// and the rest would silently not be compared at all).
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

/// A language value a person typed (or a program sent) was refused.
///
/// Carries the input back, both so the message can quote it and so a
/// caller further up (the CLI, a future editor) does not have to keep hold
/// of it separately just to report the failure — and says which of the
/// reasons in [`InputProblem`] it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInputError {
    pub input: String,
    pub problem: InputProblem,
}

/// Why a language value was refused.
///
/// Codex's catch-up review of the language-policy branch, finding 2: the
/// shared reader this crate uses (`meedya_lang::from_legacy_three_letter`)
/// is written for READING a stored field, where a zero character separates
/// several values (ID3 version 2.4, APE) and LANG-002 says the first is the
/// primary one — so it reads the first and ignores the rest. Used for a
/// value somebody is SETTING, that silently threw information away:
/// reproduced through the C API with `[{"key":"language","value":"en\u0000fr"}]`
/// (the JSON escape becomes a real zero character), which stored `en` on a
/// FLAC and `eng` on an MP3, discarded French, and answered `{"ok":true}`.
/// A value being set must be ONE value, so a zero character — or any other
/// control character, which no language code can contain — is now refused
/// with its own plain message, before anything is written, on every way in
/// (the CLI, the C API and the UniFFI API all reach this function).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputProblem {
    /// Nothing recognises it as a language.
    NotRecognised,
    /// It holds a zero character: the way a tag separates several values,
    /// so it is more than one value.
    SeveralValues,
    /// It holds this control character (other than a zero character, and
    /// other than the four whitespace characters LANG-001 step 1 trims from
    /// the two ends).
    ControlCharacter(char),
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
        //
        // The input is quoted with every control character written out as
        // `\u{..}` (`show_with_control_characters_visible`), so a refusal
        // can never print a raw zero character — which would cut the
        // message short at the C API, where text ends at a zero.
        let shown = show_with_control_characters_visible(&self.input);
        match self.problem {
            InputProblem::NotRecognised => write!(
                f,
                "'{shown}' is not a language MeedyaManager recognises. Use a language code such \
                 as \"en\", \"pt-BR\" or \"zh-Hant\" (an older three-letter code such as \
                 \"fre\" is accepted too), or \"und\" if the language is genuinely not known."
            ),
            InputProblem::SeveralValues => write!(
                f,
                "'{shown}' holds more than one value (they are separated by a zero character, \
                 shown here as \\u{{0}}). A language is set one value at a time: give just one \
                 language code, such as \"en\" or \"pt-BR\"."
            ),
            InputProblem::ControlCharacter(c) => write!(
                f,
                "'{shown}' contains a control character ({}), which no language code contains. \
                 Use a language code such as \"en\", \"pt-BR\" or \"zh-Hant\".",
                c.escape_unicode()
            ),
        }
    }
}

/// `input` with every control character written out as `\u{..}` and
/// everything else left as it is — for quoting a refused value safely.
fn show_with_control_characters_visible(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_control() {
                c.escape_unicode().to_string()
            } else {
                c.to_string()
            }
        })
        .collect()
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
/// A value being set must be ONE value (Codex's catch-up review, finding
/// 2): a zero character anywhere in it — the way a tag separates several
/// values — is refused rather than cut at the first value, and so is any
/// other control character left once LANG-001 step 1's four whitespace
/// characters (space, tab, line feed, carriage return) are trimmed from the
/// two ends. Those four at the ends are trimmed, as the policy says, so
/// `"en\n"` is still accepted as `en`; a line feed in the middle is not.
///
/// # Errors
/// Returns [`LanguageInputError`] naming the input, with an example of what
/// a valid one looks like, when nothing above recognises it, and with its
/// own explanation when it holds several values or a control character
/// (see [`InputProblem`]).
pub fn parse_language_input(input: &str) -> Result<LanguageTag, LanguageInputError> {
    let refuse = |problem: InputProblem| LanguageInputError {
        input: input.to_string(),
        problem,
    };
    if input.contains('\u{0}') {
        return Err(refuse(InputProblem::SeveralValues));
    }
    if let Some(c) = trim_lang_whitespace(input).chars().find(|c| c.is_control()) {
        return Err(refuse(InputProblem::ControlCharacter(c)));
    }
    from_legacy_three_letter(input).ok_or_else(|| refuse(InputProblem::NotRecognised))
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
    // What the person actually typed, with only LANG-001 step 1's four
    // whitespace characters taken off the ends — the same trimming the
    // shared crate does before reading anything, so a stray space never
    // makes "what was stored" look different from "what was typed".
    let typed = trim_lang_whitespace(input);
    // An old three-letter code as the first part of a longer tag
    // (`eng-Latn`) — fourth review round, item S2. See
    // `old_code_inside_longer_tag` for what counts, and why.
    let old_code = old_code_inside_longer_tag(&parsed, typed);
    // Set once the ID3 reason below has explained that old code, so the
    // crate's own note about the same subtag does not say it a second time.
    let mut old_code_explained = false;

    let mut reasons: Vec<String> = Vec::new();

    // The structural TRACK-070 reason only an ID3 tag's three-letter-only
    // field can run into — every other container this crate writes into
    // keeps the canonical tag whole, so nothing is ever lost writing into
    // one of those (a text difference there, if any, is pure
    // normalisation, covered by the notes below rather than here). Named
    // "an ID3 tag", never "an MP3" (review item 4): the same tag type is
    // reached from a `.wav` file just as often as from a `.mp3` one.
    //
    // Third review round, item 1: decided per container. When the ID3
    // tag will hold exactly what was typed (`eng` typed, `eng` stored;
    // `und` typed, `und` stored), there is nothing to explain for it at
    // all, whatever else is true of the value.
    if tag_types.contains(&TagType::Id3v2) {
        let stored = language_value_for_tag_type(&parsed, TagType::Id3v2);
        if stored.eq_ignore_ascii_case(typed) {
            // Stored exactly as typed (letter case aside) — say nothing.
        } else if let Some(old) = old_code.as_ref().filter(|_| stored == "und") {
            // Fourth review round, item S2. `eng-Latn` fell into the next
            // branch and was told "an ID3 tag has no three-letter code for
            // \"eng\" at all" — false: `eng` IS a three-letter code, the one
            // an ID3 tag uses for English. What is really going on is that
            // an old code is only understood ON ITS OWN; as the first part
            // of a longer tag it is not a language anybody recognises, so
            // there is nothing to put in the ID3 tag but "not known".
            // Reproduced with the binary built from `aa7a30d`: mutagen read
            // `und` from the MP3 after `--set language=eng-Latn`. Say so,
            // and say what to type instead.
            reasons.push(format!(
                "\"{}\" is an old code; inside a longer tag it is not recognised, so an ID3 tag \
                 will store it as not known — type \"{}\" instead",
                old.code, old.suggestion
            ));
            old_code_explained = true;
        } else if stored == "und" && parsed.language.as_deref() != Some("und") {
            // "No three-letter code" is only true of a language that is
            // not itself "not known". Before the third review round this
            // branch ran for `und` too, so typing `und` — which ID3 holds
            // perfectly well, as `und` — was reported as "an ID3 tag has
            // no three-letter code for \"und\"", which is simply false. A
            // value whose language IS `und` but which carries more (for
            // example `und-Latn`) falls through to the lost-parts branch
            // below instead, which names what is really lost.
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
            // Fourth review round, item S2: an extended-language part (the
            // `bra` of `sgn-bra`) was never listed, so typing `sgn-bra` on
            // an MP3 said nothing about losing it — only that "bra" is not
            // on the official list — while the ID3 tag stored just `sgn`.
            // It comes first because it comes first in a tag.
            if parsed.extlang.is_some() {
                dropped.push("extended-language part");
            }
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
    //
    // Third review round, item 1 ("describe only what really differs"):
    // the two "kept as typed" notes used to say "it is kept exactly as
    // typed" whatever the file was. That is true of a tag that holds the
    // full code (a FLAC's, an M4A's, a WAV's RIFF INFO chunk) and false of
    // an ID3 tag, which keeps only the three-letter language code — so on
    // an MP3, `en-JJ` was reported as losing its region AND as keeping it
    // exactly as typed, in the same sentence. The ending now says which
    // tags really do keep it, and is left off when none of them does (the
    // ID3 reason above already says what an ID3 tag stores instead).
    let keeps_whole = kept_as_typed_ending(tag_types);
    let mut note_reasons: Vec<String> = Vec::new();
    for note in &parsed.notes {
        match note {
            TagNote::UnregisteredSubtag { subtag } => {
                // Fourth review round, item S2: the old three-letter code at
                // the start of a longer tag (`eng` in `eng-Latn`) is "not on
                // the official list" too — true, but it tells nobody what
                // went wrong or what to type.
                let old_code_here = old_code
                    .as_ref()
                    .filter(|old| old.code.eq_ignore_ascii_case(subtag));
                match (old_code_here, keeps_whole) {
                    // The ID3 reason above has already explained it, and no
                    // other tag keeps anything: nothing left to add.
                    (Some(_), None) if old_code_explained => {}
                    // Not explained yet (no ID3 tag is written), and a tag
                    // keeps it whole: this note is the only place to say it.
                    (Some(old), Some(where_kept)) if !old_code_explained => {
                        note_reasons.push(format!(
                            "\"{}\" is an old code; inside a longer tag it is not recognised, \
                             but it is kept exactly as typed{where_kept} — type \"{}\" instead",
                            old.code, old.suggestion
                        ));
                    }
                    // Everything else — including the old code when the ID3
                    // reason has explained it but another tag keeps it whole —
                    // gets the note it has always had, which then only adds
                    // where it is kept.
                    (_, where_kept) => note_reasons.push(format!(
                        "\"{subtag}\" is not on the official list of language subtags{}",
                        where_kept
                            .map(|where_kept| format!(
                                ", but it is kept exactly as typed{where_kept}"
                            ))
                            .unwrap_or_default()
                    )),
                }
            }
            TagNote::DeprecatedNoReplacement { subtag } => note_reasons.push(format!(
                "\"{subtag}\" is an old code with no single replacement{}",
                keeps_whole
                    .map(|where_kept| format!(", so it is kept exactly as typed{where_kept}"))
                    .unwrap_or_default()
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
    // `SubtagReplaced` loop above already reports). Only reported when
    // nothing above already explains the difference, so a case already
    // covered (like "iw") is never reported twice.
    //
    // Third review round, item 1 (MUST FIX): this used to compare what was
    // typed against `parsed.tag` — the result of LANG-002's READER — so an
    // ordinary old three-letter code such as `eng`, `fre`, `ger` or `deu`,
    // or ID3's own "not known" marker `xxx`, was reported as "an old or
    // grouped form that is no longer used — it is replaced with the
    // current code, \"en\"". That is false twice over: those codes are
    // not retired (LANG-002 exists precisely because they are in everyday
    // use), and an ID3 tag stores `eng` exactly as typed. See
    // `is_whole_tag_replacement` for how a genuine replacement is told
    // apart now.
    if note_reasons.is_empty() && is_whole_tag_replacement(typed) {
        note_reasons.push(format!(
            "\"{typed}\" is an old or grouped form that is no longer used — it is replaced with \
             the current code, \"{}\"",
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

/// LANG-001 step 1's trim: only space, tab, line feed and carriage return,
/// from both ends — the same four characters the shared crate removes
/// before it reads anything. Deliberately NOT `str::trim`, which also
/// removes a no-break space and every other Unicode space: the shared
/// crate treats a value starting with a no-break space as malformed, and
/// this module must never tidy away something the crate would object to.
fn trim_lang_whitespace(s: &str) -> &str {
    s.trim_matches(|c: char| matches!(c, ' ' | '\t' | '\n' | '\r'))
}

/// An old three-letter code typed as the first part of a longer tag, and
/// what to type instead.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OldCodeInsideLongerTag {
    /// The old code, in lower case (`eng`).
    code: String,
    /// The same tag with the old code replaced by the current one, in its
    /// standard form (`en-Latn`).
    suggestion: String,
}

/// When `typed` starts with an old three-letter code (`eng`, `ger`, `fre`)
/// followed by more parts (`eng-Latn`, `ger-1996`, `eng-x-foo`), return the
/// code and the tag a person meant — fourth review round, item S2.
///
/// Why this needs its own message: LANG-002 understands an old code only ON
/// ITS OWN (`eng` → `en`) or in Matroska's "three letters and a country"
/// shape (`fre-CA` → `fr-CA`). Anywhere else, the shared crate reads the
/// value as an ordinary tag whose language part is `eng` — which is not a
/// language anybody has registered — so an ID3 tag stores it as "not known"
/// (`und`), and a tag that keeps the full code keeps `eng-Latn`, which
/// nothing will recognise as English later. The old note blamed ID3 for
/// having "no three-letter code for \"eng\"", which is false: `eng` is the
/// very code an ID3 tag uses for English.
///
/// Only codes LANG-002 turns into a DIFFERENT current code count. A
/// three-letter code that is already a language in its own right (`sgn`,
/// `yue`, `und`, a local-use `qaa`) is a normal first part of a longer tag,
/// and returns `None`. So does anything the shared crate already turned into
/// something else (Matroska's `fre-CA` is read as `fr-CA`), because then the
/// language part is no longer the old code at all.
///
/// The suggestion is the current code followed by the rest of what was
/// typed, put into the standard form (`fre-latn-ca` → `fr-Latn-CA`); if that
/// is somehow not a well-formed tag, the joined text is offered as it is.
fn old_code_inside_longer_tag(parsed: &LanguageTag, typed: &str) -> Option<OldCodeInsideLongerTag> {
    let (first, rest) = typed.split_once('-')?;
    if first.len() != 3 || !first.chars().all(|c| c.is_ascii_alphabetic()) || rest.is_empty() {
        return None;
    }
    // The shared crate kept it as the tag's language part — it was not
    // read some other way (as Matroska's shape is).
    if !parsed
        .language
        .as_deref()
        .is_some_and(|language| language.eq_ignore_ascii_case(first))
    {
        return None;
    }
    // On its own, LANG-002 reads it as a different, current code.
    let on_its_own = from_legacy_three_letter(first)?;
    if on_its_own.tag.eq_ignore_ascii_case(first) {
        return None;
    }
    let joined = format!("{}-{rest}", on_its_own.tag);
    let restated = canonicalise(&joined);
    Some(OldCodeInsideLongerTag {
        code: first.to_ascii_lowercase(),
        suggestion: if restated.is_malformed() {
            joined
        } else {
            restated.tag
        },
    })
}

/// Which of `tag_types` keep a language value whole, for the end of a "kept
/// exactly as typed" note (third review round, item 1).
///
/// Returns `None` when no container in the list keeps the full code — only
/// an ID3 tag, which holds just the three-letter language code — so the
/// note must not claim anything was kept as typed at all. Returns an empty
/// ending when every container keeps it, and a short qualifier when some
/// do and an ID3 tag does not.
fn kept_as_typed_ending(tag_types: &[TagType]) -> Option<&'static str> {
    let has_id3 = tag_types.contains(&TagType::Id3v2);
    let has_full = tag_types.iter().any(|tag_type| *tag_type != TagType::Id3v2);
    match (has_full, has_id3) {
        (false, _) => None,
        (true, false) => Some(""),
        (true, true) => Some(" in every tag except the ID3 one"),
    }
}

/// Whether the shared crate's LANG-001 canonicalisation REPLACES what was
/// typed with a genuinely different tag — a grandfathered tag with a
/// preferred replacement (`i-klingon` → `tlh`), a redundant combination
/// (`sgn-BR` → `bzs`), or a tag whose replaced region turns it into one of
/// those (`sgn-DD` → `gsg`) — as opposed to tidying its letter case or
/// putting its extension parts in the standard order.
///
/// Why LANG-001 (`canonicalise`) and not LANG-002 (the reader
/// [`parse_language_input`] uses): LANG-002 also turns an old three-letter
/// code into the shortest code for the same language (`eng` → `en`,
/// `ger` → `de`), turns ID3's `xxx` into `und`, and reads a Matroska-style
/// `fre-CA` as `fr-CA`. None of those is a replacement — they are the same
/// language, written the older way, in everyday use — and LANG-001 on its
/// own leaves every one of them alone (it does not know those spellings,
/// so it has nothing to replace them with). Comparing against LANG-002's
/// answer was exactly the third review round's MUST FIX: it made `eng`
/// look replaced.
///
/// Compared as a sorted set of lower-cased subtags, so a change of case
/// (`EN-gb` → `en-GB`) or of extension order (`en-u-ca-gregory-a-bbb` →
/// `en-a-bbb-u-ca-gregory`) is never mistaken for a replacement. A value
/// LANG-001 finds malformed cannot have been replaced by anything, so it
/// answers `false`.
fn is_whole_tag_replacement(typed: &str) -> bool {
    let restated = canonicalise(typed);
    if restated.is_malformed() {
        return false;
    }
    let subtags = |tag: &str| {
        let mut parts: Vec<String> = tag.split('-').map(str::to_ascii_lowercase).collect();
        parts.sort_unstable();
        parts
    };
    subtags(&restated.tag) != subtags(typed)
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
/// hope. Returns `None` when the file cannot even be probed, or when there
/// is nothing worth telling a person about. A caller that goes on to
/// actually write will get a real, specific error for a value nothing
/// recognises at all, from `write_tags` itself.
///
/// When `input` is identical to what is already shown, nothing will change
/// (review item 4 of the second round), so there is no conversion to
/// describe — but since the third review round (item 3) that is not always
/// "nothing to say": if the file's tags DISAGREE about the language (a WAV
/// whose RIFF INFO chunk says `fre` and whose ID3 tag says `ger`), resending
/// `fre` leaves the ID3 tag saying German, and a bare "✓ Set" would hide
/// that. The note then says the ID3 tag disagrees and will be left alone.
/// `write_tags` itself is NOT changed to "fix" the ID3 tag on a resend:
/// every editing screen resends every field on every save (COMPAT-030), so
/// a resend must stay a no-change. A deliberate "make every tag agree"
/// action is tracked as its own issue.
pub fn preview_conversion_note(path: &Path, input: &str) -> Option<String> {
    let tagged_file = super::open_tagged_file(path).ok()?;
    if super::current_joined_value(&tagged_file, super::TAG_LANGUAGE).as_deref() == Some(input) {
        return super::language_disagreement(&tagged_file)
            .map(|found| describe_disagreement(&found, DisagreementContext::LeftAloneByAnEdit));
    }
    let tag_types = super::language_write_targets(&tagged_file);
    describe_conversion_for_types(input, &tag_types)
}

/// A plain-English note, for someone LOOKING at a file, when its tags
/// disagree about the language — `None` when they agree, when only one kind
/// of tag holds a language, or when the file cannot be read.
///
/// Third review round, item 3: `meedya debug` and the desktop apps (through
/// `get_metadata`) show the language a tag holding the full code gives, in
/// preference to the ID3 tag's (TRACK-070). This says so when the ID3 tag
/// holds something different, so it is never silently hidden. It is a
/// report only (COMPAT-040): it changes nothing, and it does not say which
/// of the two is right — MeedyaManager cannot know that.
pub fn disagreement_note(path: &Path) -> Option<String> {
    let tagged_file = super::open_tagged_file(path).ok()?;
    super::language_disagreement(&tagged_file)
        .map(|found| describe_disagreement(&found, DisagreementContext::Reading))
}

/// Which situation a disagreement note is written for — the facts are the
/// same, but what a person needs to be told about them is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DisagreementContext {
    /// Someone is looking at the file (`meedya debug`, an app's tag list).
    Reading,
    /// An edit resent the value already shown, so nothing is written and
    /// the disagreeing ID3 tag stays as it is.
    LeftAloneByAnEdit,
}

/// Word a [`super::LanguageDisagreement`] for a person. Values are quoted
/// exactly as stored. There is no language-name data anywhere in this
/// project or the shared crate, so the note shows the codes themselves
/// (`"ger"`), not names ("German") — a name would have to be guessed.
///
/// It deliberately gives no "to fix this, do X" advice. The obvious advice
/// — "set a different language and both tags are updated" — is true for a
/// WAV, but not for a FLAC that starts with an ID3 tag, which MeedyaManager
/// cannot save at all yet (a separate, older fault with its own issue).
fn describe_disagreement(
    found: &super::LanguageDisagreement,
    context: DisagreementContext,
) -> String {
    let quote = |values: &[String]| {
        let quoted: Vec<String> = values.iter().map(|v| format!("\"{v}\"")).collect();
        let parts: Vec<&str> = quoted.iter().map(String::as_str).collect();
        join_with_and(&parts)
    };
    let hidden = quote(&found.hidden_id3);
    let shown = quote(&found.shown);
    let verb = if found.hidden_id3.len() == 1 {
        "disagrees"
    } else {
        "disagree"
    };
    match context {
        DisagreementContext::Reading => format!(
            "this file's ID3 tag says {hidden}, which {verb} with {shown} — only {shown} is \
             shown, because the tag that can hold the full language code is read first"
        ),
        DisagreementContext::LeftAloneByAnEdit => format!(
            "this file's ID3 tag says {hidden}, which {verb} with {shown} and will be left \
             alone, because {shown} is already the file's language"
        ),
    }
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

    // ── Third review round, item 1 (MUST FIX) ───────────────────────────
    //
    // Every ordinary old three-letter code used to get "... is an old or
    // grouped form that is no longer used — it is replaced with the
    // current code", because the "whole tag replaced" check compared what
    // was typed against LANG-002's reading of it. Reproduced on real files
    // with the `meedya` binary built from `e18fb18` before this fix: an
    // MP3 given `eng` stored `eng` and was told it had been replaced with
    // `en`. `metadata_roundtrip.rs` repeats these on real MP3 and FLAC
    // files and reads back what each container really holds.

    /// The container lists the ordinary cases are checked against: an ID3
    /// tag alone (an MP3), a full container alone (a FLAC), and both at
    /// once (a WAV carrying a RIFF INFO chunk AND an embedded ID3 tag).
    const EVERY_TARGET_SHAPE: [&[TagType]; 3] = [
        &[TagType::Id3v2],
        &[TagType::VorbisComments],
        &[TagType::RiffInfo, TagType::Id3v2],
    ];

    #[test]
    fn describe_conversion_is_silent_for_every_ordinary_three_letter_code() {
        // `ger` and `fre` are the bibliographic forms (an ID3 tag stores
        // the terminology forms `deu` and `fra` instead — the same
        // language, so still nothing to say); `ENG` checks letter case.
        for typed in ["eng", "fre", "ger", "deu", "fra", "ENG", " eng "] {
            for tag_types in EVERY_TARGET_SHAPE {
                assert_eq!(
                    describe_conversion_for_types(typed, tag_types),
                    None,
                    "{typed:?} on {tag_types:?}: an ordinary three-letter code is not a \
                     replacement and loses nothing"
                );
            }
        }
    }

    #[test]
    fn describe_conversion_is_silent_for_id3s_own_not_known_marker() {
        // `xxx` is ID3's own spelling of "not known"; it is stored as
        // `und`, which means exactly the same thing.
        for typed in ["xxx", "XXX"] {
            for tag_types in EVERY_TARGET_SHAPE {
                assert_eq!(
                    describe_conversion_for_types(typed, tag_types),
                    None,
                    "{typed:?} on {tag_types:?}"
                );
            }
        }
    }

    #[test]
    fn describe_conversion_never_says_und_has_no_three_letter_code() {
        // `und` IS a three-letter code, and an ID3 tag holds it as such.
        for tag_types in EVERY_TARGET_SHAPE {
            assert_eq!(
                describe_conversion_for_types("und", tag_types),
                None,
                "und on {tag_types:?}"
            );
        }
        // A value whose language is `und` but which carries more loses
        // that extra part on an ID3 tag — say THAT, not "no code".
        let note = describe_conversion_for_types("und-Latn", &[TagType::Id3v2])
            .expect("the script is lost on an ID3 tag");
        assert!(note.contains("the script you typed"), "{note}");
        assert!(note.contains("\"und\""), "{note}");
        assert!(!note.contains("no three-letter code"), "{note}");
    }

    #[test]
    fn describe_conversion_reads_a_matroska_style_code_without_calling_it_replaced() {
        // `fre-CA` is how old Matroska files give a region: LANG-002 reads
        // it as `fr-CA`. Nothing is replaced — on a full container nothing
        // is lost either; on an ID3 tag only the region is.
        assert_eq!(
            describe_conversion_for_types("fre-CA", &[TagType::VorbisComments]),
            None
        );
        let note = describe_conversion_for_types("fre-CA", &[TagType::Id3v2])
            .expect("the region is lost on an ID3 tag");
        assert!(note.contains("the region you typed"), "{note}");
        assert!(note.contains("\"fra\""), "{note}");
        assert!(!note.contains("no longer used"), "{note}");
    }

    #[test]
    fn describe_conversion_still_reports_a_region_replacement_that_becomes_a_whole_tag() {
        // `sgn-DD` → the region `DD` is replaced by `DE`, which makes the
        // tag the redundant `sgn-DE`, which is replaced whole by `gsg`.
        // The shared crate's recursion leaves no note behind, so this is
        // the whole-tag check's job.
        let note = describe_conversion_for_types("sgn-DD", &[TagType::VorbisComments])
            .expect("sgn-DD is replaced");
        assert!(
            note.contains("\"sgn-DD\"") && note.contains("\"gsg\""),
            "{note}"
        );
    }

    #[test]
    fn describe_conversion_does_not_call_a_reordering_a_replacement() {
        // LANG-001 step 6 puts extensions in order of their letter; the
        // parts are unchanged, so this is not a replacement.
        assert_eq!(
            describe_conversion_for_types("en-u-ca-gregory-a-bbb", &[TagType::VorbisComments]),
            None
        );
    }

    // ── Third review round, item 5: breakages no test used to catch ─────
    //
    // The reviewer removed, one at a time, the lines that name the script,
    // the extension part and the private-use part in the ID3 "you will
    // lose ..." note, and broke the "and" in a two-part list — and every
    // test still passed, because the only lost-part test used `pt-BR`,
    // which has a region and nothing else. Each test below names the part
    // it checks, in the exact words a person sees.

    /// Breakage N12: the script was not named.
    #[test]
    fn describe_conversion_names_a_lost_script() {
        let note = describe_conversion_for_types("zh-Hant", &[TagType::Id3v2])
            .expect("an ID3 tag cannot hold a script");
        assert!(note.contains("will lose the script you typed"), "{note}");
        assert!(note.contains("\"zho\""), "{note}");
    }

    /// Breakage N10: the extension part was not named.
    #[test]
    fn describe_conversion_names_a_lost_extension() {
        let note = describe_conversion_for_types("en-u-ca-gregory", &[TagType::Id3v2])
            .expect("an ID3 tag cannot hold an extension");
        assert!(note.contains("will lose the extension you typed"), "{note}");
        assert!(note.contains("\"eng\""), "{note}");
    }

    /// Breakage N11: the private-use part was not named.
    #[test]
    fn describe_conversion_names_a_lost_private_use_part() {
        let note = describe_conversion_for_types("en-x-mine", &[TagType::Id3v2])
            .expect("an ID3 tag cannot hold a private-use part");
        assert!(
            note.contains("will lose the private-use part you typed"),
            "{note}"
        );
    }

    /// Breakage N13: two lost parts must read "region and script", not
    /// "region, script" — and three read "a, b and c".
    #[test]
    fn describe_conversion_joins_two_or_more_lost_parts_as_a_person_would() {
        let two = describe_conversion_for_types("zh-Hant-TW", &[TagType::Id3v2])
            .expect("region and script are lost");
        assert!(
            two.contains("will lose the region and script you typed"),
            "{two}"
        );

        let three = describe_conversion_for_types("sl-Latn-IT-rozaj", &[TagType::Id3v2])
            .expect("region, script and a variant are lost");
        assert!(
            three.contains("will lose the region, script and extra detail you typed"),
            "{three}"
        );
    }

    /// Fourth review round, item S2: every input the reviewer named, and
    /// what a person should type instead.
    const OLD_CODE_INSIDE_A_LONGER_TAG: &[(&str, &str, &str)] = &[
        ("eng-Latn", "eng", "en-Latn"),
        ("ger-1996", "ger", "de-1996"),
        ("deu-1996", "deu", "de-1996"),
        ("eng-x-foo", "eng", "en-x-foo"),
        ("eng-u-ca-gregory", "eng", "en-u-ca-gregory"),
        ("fre-Latn-CA", "fre", "fr-Latn-CA"),
        ("eng-US-x-foo", "eng", "en-US-x-foo"),
    ];

    /// On an ID3 tag, an old code inside a longer tag used to be reported as
    /// "an ID3 tag has no three-letter code for \"eng\" at all" — false, as
    /// `eng` is exactly the code ID3 uses for English. The note must say
    /// what is really happening, and what to type instead — and say it
    /// once, not again as "not on the official list".
    #[test]
    fn describe_conversion_explains_an_old_code_inside_a_longer_tag_on_id3() {
        for (typed, code, suggestion) in OLD_CODE_INSIDE_A_LONGER_TAG {
            let note = describe_conversion_for_types(typed, &[TagType::Id3v2])
                .unwrap_or_else(|| panic!("{typed}: the ID3 tag stores \"not known\""));
            assert_eq!(
                note,
                format!(
                    "\"{code}\" is an old code; inside a longer tag it is not recognised, so an \
                     ID3 tag will store it as not known — type \"{suggestion}\" instead"
                ),
                "{typed}"
            );
        }
    }

    /// With no ID3 tag, the old code is kept as typed — true, and the note
    /// already said so — but the person is now also told why nothing will
    /// recognise it, and what to type instead. With both kinds of tag, the
    /// ID3 reason explains it and the other note only adds where it is kept.
    #[test]
    fn describe_conversion_explains_an_old_code_inside_a_longer_tag_everywhere() {
        for (typed, code, suggestion) in OLD_CODE_INSIDE_A_LONGER_TAG {
            let full_only = describe_conversion_for_types(typed, &[TagType::VorbisComments])
                .unwrap_or_else(|| panic!("{typed}: worth a note on a full tag too"));
            assert_eq!(
                full_only,
                format!(
                    "\"{code}\" is an old code; inside a longer tag it is not recognised, but it \
                     is kept exactly as typed — type \"{suggestion}\" instead"
                ),
                "{typed}"
            );

            let both = describe_conversion_for_types(typed, &[TagType::RiffInfo, TagType::Id3v2])
                .unwrap_or_else(|| panic!("{typed}: both notes apply"));
            assert!(
                both.starts_with(&format!(
                    "\"{code}\" is an old code; inside a longer tag it is not recognised, so an \
                     ID3 tag will store it as not known — type \"{suggestion}\" instead; "
                )),
                "{typed}: {both}"
            );
            assert!(
                both.ends_with(&format!(
                    "\"{code}\" is not on the official list of language subtags, but it is kept \
                     exactly as typed in every tag except the ID3 one"
                )),
                "{typed}: {both}"
            );
            assert!(!both.contains("no three-letter code"), "{typed}: {both}");
        }
    }

    /// The suggestion is offered in the standard form, whatever letter case
    /// was typed — "type \"en-latn\" instead" would be a second thing for
    /// the person to fix.
    #[test]
    fn describe_conversion_suggests_the_standard_form() {
        for (typed, suggestion) in [("ENG-latn", "en-Latn"), ("fre-latn-ca", "fr-Latn-CA")] {
            let note = describe_conversion_for_types(typed, &[TagType::Id3v2])
                .unwrap_or_else(|| panic!("{typed}: the ID3 tag stores \"not known\""));
            assert!(
                note.ends_with(&format!("— type \"{suggestion}\" instead")),
                "{typed}: {note}"
            );
        }
    }

    /// Not every three-letter first part is an old code. `sgn`, `yue`,
    /// `und` and `cmn` are languages in their own right, and Matroska's
    /// `fre-CA` is read as `fr-CA` — none of these may be called "an old
    /// code".
    #[test]
    fn describe_conversion_does_not_call_a_current_code_old() {
        for typed in [
            "sgn-bra", "yue-HK", "und-Latn", "cmn-Hans", "fre-CA", "en-Latn",
        ] {
            for tag_types in EVERY_TARGET_SHAPE {
                let note = describe_conversion_for_types(typed, tag_types).unwrap_or_default();
                assert!(
                    !note.contains("is an old code;"),
                    "{typed} on {tag_types:?}: {note}"
                );
            }
        }
    }

    /// Fourth review round, item S2: an extended-language part (`bra` in
    /// `sgn-bra`) was never listed among the parts an ID3 tag loses, so the
    /// note said only that "bra" is not on the official list, while the
    /// ID3 tag stored just `sgn`.
    #[test]
    fn describe_conversion_names_a_lost_extended_language_part() {
        let note = describe_conversion_for_types("sgn-bra", &[TagType::Id3v2])
            .expect("an ID3 tag cannot hold an extended-language part");
        assert!(
            note.starts_with(
                "an ID3 tag can only hold the three-letter language code, so it will lose the \
                 extended-language part you typed — it will be stored there as \"sgn\""
            ),
            "{note}"
        );
    }

    #[test]
    fn a_kept_as_typed_note_only_claims_what_each_container_really_keeps() {
        // `JJ` is not a registered region. A full container keeps it; an
        // ID3 tag drops every region. The note must never say both at once.
        let id3_only = describe_conversion_for_types("en-JJ", &[TagType::Id3v2])
            .expect("the region is lost on an ID3 tag");
        assert!(id3_only.contains("not on the official list"), "{id3_only}");
        assert!(!id3_only.contains("kept exactly as typed"), "{id3_only}");

        let full_only = describe_conversion_for_types("en-JJ", &[TagType::VorbisComments])
            .expect("an unregistered region is still worth a note");
        assert!(full_only.contains("kept exactly as typed"), "{full_only}");
        assert!(!full_only.contains("except the ID3 one"), "{full_only}");

        let both = describe_conversion_for_types("en-JJ", &[TagType::RiffInfo, TagType::Id3v2])
            .expect("both notes apply");
        assert!(
            both.contains("kept exactly as typed in every tag except the ID3 one"),
            "{both}"
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

    /// Codex's catch-up review, finding 4: a zero character separates
    /// values whatever format they came from; order is kept, each part is
    /// trimmed, and empty parts are left out — the way the ID3 path gives
    /// them.
    #[test]
    fn split_stored_values_splits_at_every_zero_and_keeps_the_order() {
        assert_eq!(split_stored_values("eng\u{0}fra"), ["eng", "fra"]);
        assert_eq!(split_stored_values("fra\u{0}eng"), ["fra", "eng"]);
        assert_eq!(split_stored_values("eng\u{0}\u{0}fra\u{0}"), ["eng", "fra"]);
        assert_eq!(split_stored_values("\u{0}en"), ["en"]);
        assert_eq!(split_stored_values(" en-GB "), ["en-GB"]);
        // Finding 5: only the policy's four whitespace characters are
        // trimmed — a no-break space (and any other Unicode space) stays.
        assert_eq!(split_stored_values("\t\r\n en \n"), ["en"]);
        assert_eq!(split_stored_values("\u{a0}en"), ["\u{a0}en"]);
        assert_eq!(split_stored_values("en\u{2003}"), ["en\u{2003}"]);
        assert_eq!(split_stored_values("\u{a0}"), ["\u{a0}"]);
        assert_eq!(split_stored_values("English"), ["English"]);
        assert!(split_stored_values("").is_empty());
        assert!(split_stored_values("\u{0}").is_empty());
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

    /// Codex's catch-up review, finding 2: a zero character means several
    /// values, and a value being set must be one. The shared reader would
    /// read `en` and drop the rest, so this must refuse first — wherever the
    /// zero is, including at the end, where a READ would treat it as padding.
    #[test]
    fn parse_language_input_refuses_several_values_rather_than_cutting_them() {
        for input in [
            "en\u{0}fr",
            "eng\u{0}fra",
            "\u{0}en",
            "en\u{0}",
            "pt-BR\u{0}pt-PT",
        ] {
            let err = parse_language_input(input).unwrap_err();
            assert_eq!(err.problem, InputProblem::SeveralValues, "{input:?}");
            let message = err.to_string();
            assert!(
                message.contains("more than one value"),
                "{input:?}: {message:?}"
            );
            assert!(
                !message.contains('\u{0}'),
                "the message must never carry a raw zero character: {message:?}"
            );
            assert!(message.contains("\\u{0}"), "the zero is shown: {message:?}");
        }
    }

    /// Any other control character is refused too, with its own message —
    /// except LANG-001 step 1's four whitespace characters at the two ends,
    /// which the policy trims.
    #[test]
    fn parse_language_input_refuses_a_control_character_but_trims_the_policys_own() {
        for input in [
            "en\u{1}fr",
            "en\nfr",
            "e\tn",
            "en\u{7f}",
            "\u{85}en",
            "\u{1b}[31men",
        ] {
            let err = parse_language_input(input).unwrap_err();
            assert!(
                matches!(err.problem, InputProblem::ControlCharacter(_)),
                "{input:?}: {:?}",
                err.problem
            );
            assert!(err.to_string().contains("control character"), "{input:?}");
        }
        for input in ["en\n", "\ten", " en\r\n"] {
            assert_eq!(parse_language_input(input).unwrap().tag, "en", "{input:?}");
        }
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
