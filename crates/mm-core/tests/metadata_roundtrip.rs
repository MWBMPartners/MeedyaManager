// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Metadata round-trip integration tests (Package 7 / FIXTURES)
//
// Everything in `crates/mm-core/src/metadata/mod.rs` and
// `crates/mm-core/src/integrity.rs` had unit-test coverage only against
// hand-fabricated bytes (a bare 44-byte WAV header, garbage "not a valid mp3
// file" strings, …). No test anywhere wrote a tag into a *real* MP3, FLAC or
// M4A container and read it back — which is exactly how the Round 1 bug in
// `integrity::write_tags_safe` (its `temp_path` produced a `.meedya_tmp`
// *suffix* extension lofty cannot resolve, so the standard write path had
// never worked on a real file) went unnoticed.
//
// This file closes that gap: it copies a tiny real fixture of each of the
// four tag containers MeedyaManager ships tag support for (see
// `tests/fixtures/README.md` for exactly how they were generated), and for
// each one exercises the full write path described in the package brief:
//
//   1. `metadata::write_tags` + `metadata::extract_tags` — the raw layer.
//   2. `integrity::write_tags_safe` with Test Mode OFF — the integrity-guarded
//      "standard" path (the one Round 1 discovered was broken).
//   3. `integrity::write_tags_safe` with Test Mode ON — asserting the
//      original is byte-identical afterwards and the `_MeedyaManager` copy
//      carries the new tags.
//
// Plus one cover-art embed/remove round trip (`embed_cover_art` /
// `remove_cover_art`, both raw and integrity-guarded).
//
// ## A genuine finding, not a test-design choice
//
// `TAG_YEAR` (mm-core's string key, mapped to lofty's `ItemKey::Year`) does
// **not** round-trip through ID3v2 (MP3) or MP4 ilst (M4A) atoms — only
// Vorbis Comments (FLAC/OGG) and legacy ID3v1 have a key mapping for
// `ItemKey::Year` in lofty 0.22's `ID3V2_MAP` / `ILST_MAP` tables. Because
// `metadata::write_tags` builds a generic `lofty::tag::Tag` and lets lofty's
// `From<Tag> for Id3v2Tag` / `From<Tag> for Ilst` conversions decide which
// items survive, an item whose key has no mapping for that container is
// **silently dropped** — no error reaches the caller. `write_tags(mp3_path,
// {TAG_YEAR: "1999"})` returns `Ok(())` having written nothing.
//
// The base tag sets below therefore deliberately omit `TAG_YEAR` for MP3 and
// M4A (FLAC keeps it), and `year_tag_does_not_round_trip_on_id3v2_or_mp4`
// pins the current (unfortunate) behaviour as an executable regression test:
// if lofty ever adds ID3v2/MP4 support for `ItemKey::Year`, that test fails
// and tells you to move `TAG_YEAR` back into `base_tags()`. The real fix
// belongs in `mm_core::metadata::mm_key_to_item_key` — e.g. mapping
// `TAG_YEAR` to `ItemKey::RecordingDate` instead, which *is* in all four
// format tables (ID3v2 `TDRC`, MP4 `©day`, WAV `ICRD`, Vorbis `DATE`) — but
// that is a design decision for whoever owns the tag registry, not something
// this test package should silently work around.
//
// License: GPL-2.0-or-later

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tempfile::TempDir;

use mm_core::integrity::{embed_cover_art_safe, remove_cover_art_safe, write_tags_safe};
use mm_core::metadata::{
    TAG_ALBUM, TAG_ALBUM_ARTIST, TAG_ARTIST, TAG_CATALOG_NUMBER, TAG_COMMENT, TAG_COMPILATION,
    TAG_COMPOSER, TAG_DISC_NUMBER, TAG_DISC_TOTAL, TAG_ENCODED_BY, TAG_GENRE, TAG_ISRC,
    TAG_LANGUAGE, TAG_TITLE, TAG_TRACK_NUMBER, TAG_TRACK_TOTAL, TAG_YEAR, TagMap, embed_cover_art,
    extract_cover_art, extract_tags, remove_cover_art, write_tags,
};
use mm_core::test_mode;

// ---------------------------------------------------------------------------
// Test-wide environment isolation
// ---------------------------------------------------------------------------
//
// `MM_CONFIG_DIR` is process-global state (an environment variable), and every
// `write_tags_safe` call consults it via `test_mode::is_enabled()` — without
// isolating it, these tests would read and mutate the *developer's real*
// MeedyaManager config directory, and parallel `#[test]` threads racing on
// the same env var would flake unpredictably. `ConfigDirGuard` mirrors the
// pattern already used in `crates/mm-core/src/integrity.rs`'s own unit tests.

/// Serialises every test in this binary that touches `MM_CONFIG_DIR`.
/// A `Mutex` (not e.g. an atomic flag) so a panicking test still releases it
/// via `Drop` during unwinding rather than poisoning the suite — the guard
/// below tolerates a poisoned lock explicitly for the same reason.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// RAII guard that points `MM_CONFIG_DIR` at a private tempdir for the
/// lifetime of one test and restores the environment on drop, even if the
/// test panics partway through (`Drop` runs during unwinding).
#[allow(unsafe_code)] // set_var/remove_var require unsafe in Edition 2024
struct ConfigDirGuard {
    // Field order matters: struct fields drop top-to-bottom, so the tempdir
    // is removed *before* the lock is released.
    _dir: TempDir,
    _lock: std::sync::MutexGuard<'static, ()>,
}

#[allow(unsafe_code)]
impl ConfigDirGuard {
    fn new() -> Self {
        let lock = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = TempDir::new().expect("failed to create MM_CONFIG_DIR tempdir");
        // SAFETY: serialised by `ENV_LOCK` above — no other thread in this
        // process can be reading/writing `MM_CONFIG_DIR` concurrently.
        unsafe {
            std::env::set_var("MM_CONFIG_DIR", dir.path());
        }
        Self {
            _dir: dir,
            _lock: lock,
        }
    }
}

#[allow(unsafe_code)]
impl Drop for ConfigDirGuard {
    fn drop(&mut self) {
        // SAFETY: see `new()` — still under `ENV_LOCK` (held by `_lock` until
        // this `Drop` returns).
        unsafe {
            std::env::remove_var("MM_CONFIG_DIR");
        }
    }
}

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

/// The committed fixture directory — see `tests/fixtures/README.md` for
/// exactly which `ffmpeg` command produced each file.
fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Copy a named fixture into a fresh tempdir so every test mutates its own
/// private throwaway copy — the committed fixture in the repo is never
/// touched. Returns the `TempDir` alongside the copied path; the caller must
/// keep the `TempDir` alive for as long as the path is used (it deletes the
/// directory on drop).
fn copy_fixture(name: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("failed to create fixture tempdir");
    let dest = dir.path().join(name);
    fs::copy(fixtures_dir().join(name), &dest)
        .unwrap_or_else(|e| panic!("failed to copy fixture '{name}' into tempdir: {e}"));
    (dir, dest)
}

/// Build a `TagMap` from `(key, value)` pairs, one value per key — a thin
/// convenience over the `HashMap<String, Vec<String>>` shape `write_tags`
/// expects.
fn build_tags(pairs: &[(&str, &str)]) -> TagMap {
    let mut map = TagMap::new();
    for (key, value) in pairs {
        map.insert((*key).to_string(), vec![(*value).to_string()]);
    }
    map
}

/// Assert every key in `expected` is present in `actual` with the exact same
/// value(s) — i.e. that a metadata write survived a round trip through a real
/// tag container.
fn assert_tags_survived(context: &str, expected: &TagMap, actual: &TagMap) {
    for (key, values) in expected {
        assert_eq!(
            actual.get(key).map(Vec::as_slice),
            Some(values.as_slice()),
            "{context}: tag '{key}' did not survive the round trip \
             (expected {values:?}, got {:?})",
            actual.get(key)
        );
    }
}

// ---------------------------------------------------------------------------
// Per-format tag sets
// ---------------------------------------------------------------------------

/// The tag set verified (against lofty 0.22's `ID3V2_MAP` / `ILST_MAP` key
/// tables) to have a real frame/atom mapping in both ID3v2 (MP3) and MP4 ilst
/// (M4A). Deliberately excludes `TAG_YEAR` and `TAG_BPM` — see this file's
/// module doc comment for `TAG_YEAR`; `TAG_BPM` maps to `ItemKey::Bpm`, which
/// ID3v2 only exposes as the *different* `ItemKey::IntegerBpm` (frame
/// `TBPM`), so it has the same silent-drop problem and is out of scope for
/// this package.
fn base_tags(prefix: &str) -> TagMap {
    build_tags(&[
        (TAG_TITLE, &format!("{prefix} Title")),
        (TAG_ARTIST, &format!("{prefix} Artist")),
        (TAG_ALBUM, &format!("{prefix} Album")),
        (TAG_ALBUM_ARTIST, &format!("{prefix} Album Artist")),
        (TAG_GENRE, "Ambient"),
        (TAG_TRACK_NUMBER, "3"),
        (TAG_TRACK_TOTAL, "12"),
        (TAG_DISC_NUMBER, "1"),
        (TAG_DISC_TOTAL, "2"),
        (TAG_COMPOSER, &format!("{prefix} Composer")),
        (TAG_COMMENT, &format!("{prefix} round-trip comment")),
        (TAG_COMPILATION, "1"),
        (TAG_ISRC, "GBUM71029601"),
        (TAG_CATALOG_NUMBER, "CAT-001"),
        (TAG_LANGUAGE, "en"),
        (TAG_ENCODED_BY, "MeedyaManager test suite"),
    ])
}

/// FLAC (Vorbis Comments) supports everything in `base_tags` plus `TAG_YEAR`
/// (Vorbis's `YEAR` field maps cleanly to `ItemKey::Year` — see the module
/// doc comment for why MP3/M4A do not get the same treatment).
fn flac_tags() -> TagMap {
    let mut tags = base_tags("FLAC");
    tags.insert(TAG_YEAR.to_string(), vec!["1999".to_string()]);
    tags
}

/// WAV (RIFF INFO) is the narrowest container MeedyaManager supports: no
/// album-artist, disc number/total, compilation flag, ISRC, catalogue number
/// or language-independent encoder field exist in lofty's `RIFF_INFO_MAP` —
/// only the fields below have a chunk-id mapping.
fn wav_tags() -> TagMap {
    build_tags(&[
        (TAG_TITLE, "WAV Title"),
        (TAG_ARTIST, "WAV Artist"),
        (TAG_ALBUM, "WAV Album"),
        (TAG_TRACK_NUMBER, "3"),
        (TAG_TRACK_TOTAL, "12"),
        (TAG_GENRE, "Ambient"),
        (TAG_COMPOSER, "WAV Composer"),
        (TAG_COMMENT, "WAV round-trip comment"),
        (TAG_LANGUAGE, "en"),
        (TAG_ENCODED_BY, "MeedyaManager test suite"),
    ])
}

// ---------------------------------------------------------------------------
// The shared round-trip sequence (package brief steps 1-5)
// ---------------------------------------------------------------------------

/// Run every write path the brief asks for against one fixture/tag-set pair.
/// The read-back is checked against `tags` itself — the ordinary case,
/// where every value written is expected to come back unchanged.
///
/// 1. `write_tags` + `extract_tags` (the raw metadata layer).
/// 2. `write_tags_safe` with Test Mode OFF (the integrity-guarded standard
///    path — the one that had never worked on a real file before Round 1).
/// 3. `write_tags_safe` with Test Mode ON — the original must come out
///    byte-identical and the `_MeedyaManager` copy must carry the new tags.
fn round_trip_all_paths(fixture: &str, tags: &TagMap) {
    round_trip_all_paths_expecting(fixture, tags, tags);
}

/// As [`round_trip_all_paths`], but checks the read-back tags against
/// `expected` rather than `tags` itself.
///
/// This split exists for `TAG_LANGUAGE` (policy MWBM-MEDIA-LANG 1.0.0,
/// TRACK-070): ID3v2's `TLAN` frame has no field that can hold a full BCP
/// 47 tag at all, only the old three-letter ISO 639-2 code, so a value
/// written into an MP3 is not always the same value read back out of it —
/// see `mp3_id3v2_round_trip` below for the concrete case. Every other tag
/// this crate writes, and `language` on every other container, still
/// round-trips unchanged, which is why every OTHER call site keeps using
/// the single-argument [`round_trip_all_paths`].
fn round_trip_all_paths_expecting(fixture: &str, tags: &TagMap, expected: &TagMap) {
    // Isolate MM_CONFIG_DIR for the whole sequence: even step 1's plain
    // `write_tags` call doesn't touch it, but steps 2-3 (`write_tags_safe`)
    // both consult `test_mode::is_enabled()`, so the guard must already be in
    // place before either runs.
    let _guard = ConfigDirGuard::new();

    // === 1. Raw metadata layer =============================================
    {
        let (_dir, path) = copy_fixture(fixture);
        write_tags(&path, tags).unwrap_or_else(|e| panic!("{fixture}: write_tags failed: {e}"));
        let read_back =
            extract_tags(&path).unwrap_or_else(|e| panic!("{fixture}: extract_tags failed: {e}"));
        assert_tags_survived(fixture, expected, &read_back);
    }

    // === 2. write_tags_safe, Test Mode OFF =================================
    {
        let (_dir, path) = copy_fixture(fixture);
        let result = write_tags_safe(&path, tags);
        assert!(
            result.success,
            "{fixture}: write_tags_safe (Test Mode off) failed: {:?}",
            result.error
        );
        assert_eq!(
            result.path, path,
            "{fixture}: outside Test Mode the result must name the original path"
        );
        assert_ne!(
            result.sha256_after.as_deref(),
            Some(result.sha256_before.as_str()),
            "{fixture}: the file must actually have changed"
        );
        let read_back = extract_tags(&path).unwrap_or_else(|e| {
            panic!("{fixture}: file unreadable after write_tags_safe (Test Mode off): {e}")
        });
        assert_tags_survived(fixture, expected, &read_back);
    }

    // === 3. write_tags_safe, Test Mode ON ===================================
    {
        let (_dir, path) = copy_fixture(fixture);
        let original_bytes = fs::read(&path).expect("fixture copy must be readable");

        test_mode::enable().unwrap_or_else(|e| panic!("{fixture}: test_mode::enable failed: {e}"));

        let result = write_tags_safe(&path, tags);
        assert!(
            result.success,
            "{fixture}: write_tags_safe (Test Mode on) failed: {:?}",
            result.error
        );

        let copy_path = test_mode::test_mode_path(&path);
        assert_eq!(
            result.path, copy_path,
            "{fixture}: Test Mode must divert the write to the _MeedyaManager copy"
        );

        assert_eq!(
            fs::read(&path).expect("original must still exist after a Test Mode write"),
            original_bytes,
            "{fixture}: the ORIGINAL must be byte-identical after a Test Mode write"
        );

        assert!(
            copy_path.exists(),
            "{fixture}: the _MeedyaManager copy must exist after a Test Mode write"
        );
        let copy_tags = extract_tags(&copy_path)
            .unwrap_or_else(|e| panic!("{fixture}: _MeedyaManager copy unreadable: {e}"));
        assert_tags_survived(fixture, expected, &copy_tags);

        test_mode::disable()
            .unwrap_or_else(|e| panic!("{fixture}: test_mode::disable failed: {e}"));
    }
}

// ---------------------------------------------------------------------------
// One test per real tag container
// ---------------------------------------------------------------------------

#[test]
fn mp3_id3v2_round_trip() {
    // `base_tags` writes `TAG_LANGUAGE: "en"`, but ID3v2's `TLAN` frame
    // (policy MWBM-MEDIA-LANG 1.0.0, TRACK-070) has no field able to hold a
    // BCP 47 tag at all — only the old three-letter ISO 639-2 form. Reading
    // it back therefore gives "eng", not "en"; see
    // `language_pt_br_round_trips_per_container_track_070` below for a
    // language whose three-letter form actually differs by the choice of
    // bibliographic vs. terminology, which "eng" does not (English is the
    // same string in both).
    let tags = base_tags("MP3");
    let mut expected = tags.clone();
    expected.insert(TAG_LANGUAGE.to_string(), vec!["eng".to_string()]);
    round_trip_all_paths_expecting("silence.mp3", &tags, &expected);
}

#[test]
fn flac_vorbis_round_trip() {
    round_trip_all_paths("silence.flac", &flac_tags());
}

#[test]
fn m4a_ilst_round_trip() {
    round_trip_all_paths("silence.m4a", &base_tags("M4A"));
}

#[test]
fn wav_write_tags_uses_embedded_id3v2_not_riff_info() {
    // A genuine surprise found while building the language-policy work
    // (TRACK-070's footer: "each project MUST check [tool-specific details]
    // against the tool it actually runs, with a test"): despite this
    // test's own name, and despite `wav_tags`'s doc comment above, a fresh
    // `silence.wav` — one `write_tags` has never touched before — does NOT
    // get a RIFF INFO tag from `write_tags` at all. Lofty's own
    // `FileType::primary_tag_type()` table (`lofty::file::file_type`) maps
    // `FileType::Wav` to `TagType::Id3v2`, the same as MP3, and
    // `get_or_create_primary_tag` in this crate always asks for exactly
    // that "primary" type — so it creates an ID3v2 tag embedded inside the
    // WAV container, not a RIFF `LIST INFO` chunk. `TAG_LANGUAGE: "en"`
    // therefore comes back as the ID3 terminology three-letter form "eng",
    // for exactly the reason `mp3_id3v2_round_trip` above does. This is a
    // pre-existing fact about how this crate writes WAV files generally
    // (every OTHER tag written here lands in the same embedded ID3v2 tag,
    // not RIFF INFO), not something this policy work changed — but the
    // language-specific write path is the first thing to depend on WHICH
    // container a file is actually using, which is what surfaced it.
    let tags = wav_tags();
    let mut expected = tags.clone();
    expected.insert(TAG_LANGUAGE.to_string(), vec!["eng".to_string()]);
    round_trip_all_paths_expecting("silence.wav", &tags, &expected);
}

/// Item 5 of the language-policy review: a file can carry a language value
/// in MORE THAN ONE tag container at once — `riff_language.wav` (a real
/// `ffmpeg`-made file, `-metadata language=fre`, see `tests/fixtures/
/// README.md`) has RIFF INFO's own `ILNG=fre` from the moment it is
/// created, and (per the test above) `write_tags` puts a NEW `language`
/// value into an embedded ID3v2 tag, never RIFF INFO. Before this fix,
/// setting `language=en` therefore left the file with a NEW, correct `en`
/// in the ID3v2 tag it had just created AND a STALE, now-contradicting
/// `fre` still sitting in RIFF INFO — reading the file back genuinely gave
/// `["fre", "eng"]`, two different answers for the same field. `write_tags`
/// now keeps every container that ALREADY has a language value consistent
/// with a genuinely new one, each in its own correct per-format form.
#[test]
fn setting_language_keeps_every_tag_container_consistent() {
    let (_dir, path) = copy_fixture("riff_language.wav");

    // Sanity check on the fixture itself: RIFF INFO's ILNG really is there
    // before anything touches it, and — per the test above — the primary
    // (ID3v2) tag does not have a language value of its own yet.
    let before = extract_tags(&path).unwrap();
    assert_eq!(
        before.get(TAG_LANGUAGE).map(Vec::as_slice),
        Some(["fre".to_string()].as_slice()),
        "fixture sanity check: riff_language.wav must start with only RIFF INFO's ILNG=fre"
    );

    let mut tags = TagMap::new();
    tags.insert(TAG_LANGUAGE.to_string(), vec!["en".to_string()]);
    write_tags(&path, &tags).unwrap_or_else(|e| panic!("write_tags failed: {e}"));

    // -- Physical check: BOTH underlying containers are actually kept in
    // sync, each in its own correct TRACK-070 form — read directly,
    // bypassing `extract_tags`' own cross-container merge logic entirely,
    // so this half of the test cannot be fooled by a bug in the very code
    // the second half below is checking.
    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::RiffInfo).as_deref(),
        Some("en"),
        "RIFF INFO must carry the canonical tag — no stale \"fre\" left behind"
    );
    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::Id3v2).as_deref(),
        Some("eng"),
        "the newly created ID3v2 tag must carry the terminology three-letter code"
    );

    // -- What `extract_tags` itself reports: review item 5 of the SECOND
    // review round found this reporting BOTH representations —
    // `["en", "eng"]` — as if they were two different facts, rather than
    // the SAME fact told twice in two containers. TRACK-070's own
    // priority (a full-tag container's answer is read on its own; ID3's
    // narrower one is only read when there is no full-tag answer to
    // prefer — see `read_language_values` in `metadata/mod.rs`) means
    // only RIFF INFO's answer is reported here.
    let after = extract_tags(&path)
        .unwrap_or_else(|e| panic!("extract_tags failed after setting language: {e}"));
    assert_eq!(
        after.get(TAG_LANGUAGE).map(Vec::as_slice),
        Some(["en".to_string()].as_slice()),
        "extract_tags must report the fact ONCE, not the same language told twice in two forms"
    );
}

/// Reads whatever `language` value ONE specific tag container holds,
/// bypassing every bit of `extract_tags`' own cross-container merge and
/// priority logic — used only to prove, independently of that logic, what
/// is ACTUALLY sitting in each of a file's containers on disk.
fn read_raw_language_from_tag_type(path: &Path, tag_type: lofty::tag::TagType) -> Option<String> {
    use lofty::file::TaggedFileExt;
    use lofty::probe::Probe;
    use lofty::tag::{ItemKey, ItemValue};

    let tagged_file = Probe::open(path)
        .unwrap_or_else(|e| panic!("{}: cannot probe: {e}", path.display()))
        .read()
        .unwrap_or_else(|e| panic!("{}: cannot read: {e}", path.display()));
    for tag in tagged_file.tags() {
        if tag.tag_type() == tag_type {
            for item in tag.get_items(&ItemKey::Language) {
                if let ItemValue::Text(text) = item.value() {
                    return Some(text.clone());
                }
            }
        }
    }
    None
}

/// Review item 5 of the second review round, the other half: setting
/// language on a file where TWO different languages were already sitting
/// in two different containers must still leave exactly the RIFF INFO
/// answer as `extract_tags`' report — this is not specific to the
/// "starts with one value" case above, which could in principle have been
/// coincidence.
#[test]
fn setting_language_replaces_both_containers_even_when_they_already_disagreed() {
    let (_dir, path) = copy_fixture("riff_language.wav");
    // Give the (freshly created) ID3v2 tag a DIFFERENT stored language
    // than RIFF INFO's own "fre", by writing "de" through this crate's own
    // per-format-correct path first.
    let mut first = TagMap::new();
    first.insert(TAG_LANGUAGE.to_string(), vec!["de".to_string()]);
    write_tags(&path, &first).unwrap_or_else(|e| panic!("first write_tags failed: {e}"));
    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::RiffInfo).as_deref(),
        Some("de"),
        "sanity check: the first write must have reached RIFF INFO"
    );
    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::Id3v2).as_deref(),
        Some("deu"),
        "sanity check: the first write must have reached the new ID3v2 tag too"
    );

    // Now set a SECOND, different language.
    let mut second = TagMap::new();
    second.insert(TAG_LANGUAGE.to_string(), vec!["pt-BR".to_string()]);
    write_tags(&path, &second).unwrap_or_else(|e| panic!("second write_tags failed: {e}"));

    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::RiffInfo).as_deref(),
        Some("pt-BR"),
        "RIFF INFO must carry the NEW canonical tag, not the first write's \"de\""
    );
    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::Id3v2).as_deref(),
        Some("por"),
        "the ID3v2 tag must carry the NEW terminology code — a region-dropping conversion, but \
         of the NEW value, not a leftover of the old one"
    );

    let after = extract_tags(&path).unwrap_or_else(|e| panic!("extract_tags failed: {e}"));
    assert_eq!(
        after.get(TAG_LANGUAGE).map(Vec::as_slice),
        Some(["pt-BR".to_string()].as_slice()),
        "extract_tags must report the one, current fact — not the old language, and not both \
         forms of the new one"
    );
}

/// Review item 3 of the second review round, MUST FIX: `write_tags`'
/// `LanguageChange::Clear` branch only ever removed `language` from the
/// PRIMARY tag — `remove_tag` (the general "delete this key" command) has
/// always looped over every container the file has, so a language CLEAR
/// silently doing less than an ordinary field's removal was the
/// surprising direction for the two to have drifted apart in.
#[test]
fn clearing_language_removes_it_from_every_container() {
    let (_dir, path) = copy_fixture("riff_language.wav");

    // Give the (freshly created) ID3v2 tag its own language value too, so
    // there are genuinely two containers to clear.
    let mut set = TagMap::new();
    set.insert(TAG_LANGUAGE.to_string(), vec!["en".to_string()]);
    write_tags(&path, &set).unwrap_or_else(|e| panic!("setup write_tags failed: {e}"));
    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::RiffInfo).as_deref(),
        Some("en"),
        "sanity check: RIFF INFO must carry a value before it can be cleared"
    );
    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::Id3v2).as_deref(),
        Some("eng"),
        "sanity check: the ID3v2 tag must carry a value before it can be cleared"
    );

    // Clear it — an empty value is how `write_tags` spells "remove this".
    let mut clear = TagMap::new();
    clear.insert(TAG_LANGUAGE.to_string(), vec![String::new()]);
    write_tags(&path, &clear).unwrap_or_else(|e| panic!("clearing write_tags failed: {e}"));

    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::RiffInfo),
        None,
        "RIFF INFO must no longer carry a language value after clearing"
    );
    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::Id3v2),
        None,
        "the ID3v2 tag must no longer carry a language value after clearing"
    );

    let after = extract_tags(&path).unwrap_or_else(|e| panic!("extract_tags failed: {e}"));
    assert_eq!(
        after.get(TAG_LANGUAGE),
        None,
        "extract_tags must report no language at all once every container has been cleared"
    );
}

// ---------------------------------------------------------------------------
// The TAG_YEAR finding — pinned as an executable regression test
// ---------------------------------------------------------------------------

#[test]
fn year_tag_does_not_round_trip_on_id3v2_or_mp4() {
    // See the module doc comment. No `ConfigDirGuard` needed — this only
    // exercises the raw `metadata::write_tags` / `extract_tags` layer, which
    // never consults Test Mode.
    for fixture in ["silence.mp3", "silence.m4a"] {
        let (_dir, path) = copy_fixture(fixture);

        let mut tags = TagMap::new();
        tags.insert(TAG_YEAR.to_string(), vec!["1999".to_string()]);

        // The write itself must still succeed — lofty drops the unmappable
        // item rather than erroring the whole write.
        write_tags(&path, &tags).unwrap_or_else(|e| panic!("{fixture}: write_tags failed: {e}"));

        let read_back =
            extract_tags(&path).unwrap_or_else(|e| panic!("{fixture}: extract_tags failed: {e}"));

        assert!(
            !read_back.contains_key(TAG_YEAR),
            "{fixture}: TAG_YEAR unexpectedly round-tripped (got {:?}) — lofty \
             must have gained ID3v2/MP4 support for ItemKey::Year; move \
             TAG_YEAR into base_tags() for this format and delete this test",
            read_back.get(TAG_YEAR)
        );
    }
}

// ---------------------------------------------------------------------------
// Policy MWBM-MEDIA-LANG 1.0.0 — TRACK-070 per-container writing,
// and COMPAT-030 (an untouched value is never rewritten)
// ---------------------------------------------------------------------------

/// TRACK-070's own worked example: a region only survives writing in a
/// container that has a genuine slot for a full BCP 47 tag. ID3's `TLAN`
/// frame does not — it can only ever hold the old three-letter ISO 639-2
/// code — so `"pt-BR"` written there loses its region on the way in, by
/// design, and comes back as `"por"` (Portuguese does not differ between
/// its bibliographic and terminology forms, so this is also the
/// terminology form `language_value_for_tag_type` would have written).
/// Vorbis and the MP4 freeform item both have a genuine free-text slot, so
/// the canonical tag survives whole there. `silence.wav` is ID3v2 too, for
/// the reason explained at length on `wav_write_tags_uses_embedded_id3v2_not_riff_info` above — a
/// fresh WAV file gets the same embedded ID3v2 tag an MP3 does, not a RIFF
/// INFO chunk, so it takes the same lossy path as `silence.mp3`. No
/// `ConfigDirGuard` is needed — this exercises only the raw `write_tags` /
/// `extract_tags` layer, which never consults Test Mode (same reasoning as
/// `year_tag_does_not_round_trip_on_id3v2_or_mp4` above).
#[test]
fn language_pt_br_round_trips_per_container_track_070() {
    let cases: &[(&str, &str)] = &[
        ("silence.mp3", "por"),    // ID3 TLAN: old three-letter form only
        ("silence.flac", "pt-BR"), // Vorbis LANGUAGE: the canonical tag
        ("silence.m4a", "pt-BR"),  // MP4 freeform LANGUAGE item: the canonical tag
        ("silence.wav", "por"),    // Embedded ID3v2 TLAN (see comment above): old three-letter form
    ];

    for (fixture, expected_raw) in cases {
        let (_dir, path) = copy_fixture(fixture);
        let tags = build_tags(&[(TAG_LANGUAGE, "pt-BR")]);
        write_tags(&path, &tags).unwrap_or_else(|e| panic!("{fixture}: write_tags failed: {e}"));

        let read_back =
            extract_tags(&path).unwrap_or_else(|e| panic!("{fixture}: extract_tags failed: {e}"));
        assert_eq!(
            read_back.get(TAG_LANGUAGE).map(Vec::as_slice),
            Some([expected_raw.to_string()].as_slice()),
            "{fixture}: raw stored 'language' after writing \"pt-BR\" — got {:?}, expected {expected_raw:?}",
            read_back.get(TAG_LANGUAGE)
        );
    }

    // And LANG-002's reader turns the lossy ID3 form back into the primary
    // language it still names, "pt" — the region is genuinely gone, not
    // silently guessed back, exactly as TRACK-070 warns it would be.
    let recovered = mm_core::metadata::language::parse_stored_language("por");
    assert_eq!(recovered.tag.tag, "pt");
}

/// Review item 11 of issue #251's independent review: the test above proves
/// TRACK-070's "MP3 only gets the three-letter code" rule, but Portuguese's
/// bibliographic ("por") and terminology ("por") forms are IDENTICAL, so it
/// cannot prove `write_tags` reaches for the terminology form specifically
/// rather than the bibliographic one by coincidence. German's two forms
/// differ ("ger" bibliographic, "deu" terminology per ISO 639-2, and
/// confirmed against this project's own copy of the policy fixture,
/// `write-01` in `tests/fixtures/bcp47-language-policy-v1.json`), so a real
/// MP3 round trip with "de" is the one case that actually distinguishes the
/// two — this is `language_value_for_tag_type_matches_the_policy_fixture_
/// for_id3v2` in `media_language_conformance.rs` again, but against a real
/// file written and read back through lofty, not the function called
/// directly.
#[test]
fn language_de_round_trips_to_the_terminology_form_on_id3v2() {
    let (_dir, path) = copy_fixture("silence.mp3");
    let tags = build_tags(&[(TAG_LANGUAGE, "de")]);
    write_tags(&path, &tags).unwrap_or_else(|e| panic!("write_tags failed: {e}"));

    let read_back = extract_tags(&path).unwrap_or_else(|e| panic!("extract_tags failed: {e}"));
    assert_eq!(
        read_back.get(TAG_LANGUAGE).map(Vec::as_slice),
        Some(["deu".to_string()].as_slice()),
        "raw stored 'language' after writing \"de\" to an MP3 — got {:?}, expected the \
         terminology form \"deu\", NOT the bibliographic form \"ger\"",
        read_back.get(TAG_LANGUAGE)
    );
}

/// Writes a language value straight into a file's tag, bypassing
/// `write_tags`'s own validation entirely — standing in for a value that
/// arrived some other way (an older build of this app before this policy
/// existed, another tool such as MusicBrainz Picard, or a hand-edited
/// file), which is precisely the kind of value COMPAT-030 exists to
/// protect: `write_tags` must never "fix" it just because the file was
/// opened for some other reason.
fn poke_raw_language_value(path: &Path, raw: &str) {
    use lofty::config::WriteOptions;
    use lofty::file::TaggedFileExt;
    use lofty::probe::Probe;
    use lofty::tag::{ItemKey, ItemValue, Tag, TagExt, TagItem};

    let mut tagged_file = Probe::open(path)
        .unwrap_or_else(|e| panic!("{}: cannot probe: {e}", path.display()))
        .read()
        .unwrap_or_else(|e| panic!("{}: cannot read: {e}", path.display()));
    if tagged_file.primary_tag_mut().is_none() {
        let tag_type = tagged_file.primary_tag_type();
        tagged_file.insert_tag(Tag::new(tag_type));
    }
    let tag = tagged_file
        .primary_tag_mut()
        .expect("primary tag must exist after insert_tag");
    tag.remove_key(&ItemKey::Language);
    tag.push(TagItem::new(
        ItemKey::Language,
        ItemValue::Text(raw.to_string()),
    ));
    tag.save_to_path(path, WriteOptions::default())
        .unwrap_or_else(|e| panic!("{}: cannot save: {e}", path.display()));
}

/// COMPAT-030: "Valid existing language data ... MUST be preserved when a
/// file or record is touched for another reason." This also covers the
/// harder half of the same rule — a value this crate cannot even PARSE
/// must not be lost either, as long as nobody asked to change it — because
/// `write_tags` only ever converts `language` when the caller's map
/// actually contains that key (see this crate's own doc comment on
/// `write_tags` for the "rebuilds the whole tag map" caution this test is
/// meant to guard against).
#[test]
fn compat_030_an_untouched_language_value_survives_byte_for_byte() {
    let cases: &[(&str, &str)] = &[
        ("silence.mp3", "fre"),  // a real, if non-canonical, legacy value
        ("silence.flac", "zzz"), // not recognised at all — must survive anyway
    ];

    for (fixture, raw) in cases {
        let (_dir, path) = copy_fixture(fixture);
        poke_raw_language_value(&path, raw);

        // Touch the file for an ENTIRELY DIFFERENT reason. `language` is
        // deliberately left out of this map — that omission is what
        // COMPAT-030 relies on.
        let mut other_field = TagMap::new();
        other_field.insert(TAG_TITLE.to_string(), vec!["Retitled".to_string()]);
        write_tags(&path, &other_field)
            .unwrap_or_else(|e| panic!("{fixture}: write_tags failed: {e}"));

        let read_back =
            extract_tags(&path).unwrap_or_else(|e| panic!("{fixture}: extract_tags failed: {e}"));
        assert_eq!(
            read_back.get(TAG_LANGUAGE).map(Vec::as_slice),
            Some([raw.to_string()].as_slice()),
            "{fixture}: an untouched 'language' value must survive a save made for another \
             reason byte for byte (COMPAT-030) — got {:?}, expected {raw:?} unchanged",
            read_back.get(TAG_LANGUAGE)
        );
        assert_eq!(
            read_back.get(TAG_TITLE).map(Vec::as_slice),
            Some(["Retitled".to_string()].as_slice()),
            "{fixture}: the field that WAS deliberately changed must still have taken effect"
        );
    }
}

/// Review item 2 of the second language-policy review round, MUST FIX: the
/// test above (and every other COMPAT-030 test in this file until now)
/// proves "the caller LEFT `language` OUT of the map" is a no-op — but
/// `language` being left out takes a completely different code path
/// through `write_tags` (`tags.get(TAG_LANGUAGE)` is `None`, so
/// `language_change` is `None` immediately, never even reaching the
/// identical-value comparison at all) than "the caller RESENT the exact
/// value that is already there", which is what every native UI this
/// project has actually does on every save (see `write_tags`'s own doc
/// comment). An independent review's own mutation testing found that
/// disabling that comparison outright still passed every existing test in
/// this file, because none of them exercised it. This test does, with the
/// four cases the review specified — two on a FULL container (FLAC) that
/// could never have produced them itself, one on ID3 with a value TRACK-070
/// says ID3 should never hold in the first place, and one nothing
/// recognises at all — because a resend must leave ANY already-stored
/// value alone, not only one this crate would have chosen to write.
#[test]
fn resending_an_unchanged_language_value_is_never_refused() {
    let cases: &[(&str, &str)] = &[
        // A plain English WORD, not a language code at all — exactly the
        // FLAC-says-"English" case that motivated COMPAT-030 in the first
        // place (see `write_tags`'s own doc comment). Confirmed refused if
        // typed fresh: `parse_language_input("English")` is `Err`.
        ("silence.flac", "English"),
        // The OLD three-letter form, sitting in a container that could
        // hold the full tag instead — FLAC's Vorbis comment. `write_tags`
        // itself would never put "eng" there (it writes the canonical tag
        // to a full container), but nothing stops another tool, or an
        // older build, from having done so.
        ("silence.flac", "eng"),
        // A full BCP-47 tag with a region, sitting in ID3 — the one
        // container TRACK-070 says can only ever hold the bare
        // three-letter code. `write_tags` would never put "pt-BR" there
        // either, but a resent value must be left exactly as it already
        // is, never "corrected" towards what this crate would have chosen.
        ("silence.mp3", "pt-BR"),
        // Nothing recognises this at all — confirmed refused if typed
        // fresh, the same as "English" above, by a different route
        // (`from_legacy_three_letter("zzz")` is `None` even though
        // `canonicalise("zzz")` alone parses it as an obscure ordinary
        // tag; LANG-002's three-letter-specific step is stricter).
        ("silence.flac", "zzz"),
    ];

    for (fixture, raw) in cases {
        let (_dir, path) = copy_fixture(fixture);
        poke_raw_language_value(&path, raw);
        let before = std::fs::read(&path).unwrap_or_else(|e| panic!("{fixture}: read failed: {e}"));

        // RESEND the exact value that is already there, with the key
        // present — never omitted — which is the shape the identical-value
        // comparison itself exists to handle.
        let mut tags = TagMap::new();
        tags.insert(TAG_LANGUAGE.to_string(), vec![(*raw).to_string()]);
        write_tags(&path, &tags).unwrap_or_else(|e| {
            panic!("{fixture}: resending the unchanged value {raw:?} must not be refused: {e}")
        });

        let after = std::fs::read(&path).unwrap_or_else(|e| panic!("{fixture}: read failed: {e}"));
        assert_eq!(
            before, after,
            "{fixture}: resending {raw:?} unchanged must not touch the file at all, not even \
             re-write it with the same bytes (COMPAT-030)"
        );
    }
}

/// LANG-002: "If the field holds several values (ID3v2.4 separates them
/// with a null character), split them first and read each on its own; the
/// first is the primary language."
///
/// **A genuine surprise, found by writing this test rather than assuming
/// it**: a real MP3's `TLAN` frame is poked directly with a null-separated
/// two-value shape (`"eng\0swe"`, a real null byte) — the same low-level
/// route `poke_raw_language_value` uses for the COMPAT-030 test above — and
/// the null character genuinely does reach the file on disk (confirmed by
/// reading the raw bytes back: `65 6e 67 00 73 77 65`, "eng", a null, then
/// "swe"). But `lofty`'s OWN ID3v2 reader does not hand that back as ONE
/// string with an embedded null, the way `poke_raw_language_value`'s write
/// call took it in — it already splits an ID3v2.4 multi-value text frame
/// into SEPARATE items on read, exactly like it already does for any other
/// multi-valued ID3 frame (several artists, say). So `extract_tags` on a
/// file like this returns `language: ["eng", "swe"]` — a plain two-element
/// vector, not a single string a caller has to know to split.
///
/// This means MeedyaManager's own code never actually needs to perform
/// LANG-002's null-splitting step itself when reading through `lofty` —
/// the generic multi-value machinery `read_tag_into_map` already has for
/// every tag key does it for free. LANG-002's "the first is the primary
/// language" rule becomes, in practice: take the first element of the
/// vector. `meedya_lang::from_legacy_three_letter`'s OWN null-splitting
/// (exercised directly, on a single string, by this crate's unit test
/// `parse_stored_language_reads_only_the_primary_of_a_multi_value_tlan`)
/// is still correct and still worth having — it matters for a caller that
/// receives a raw value some OTHER way than through `lofty`'s already-split
/// API (a hand-parsed ID3 buffer, another library) — but it is not what
/// runs for a value that reaches MeedyaManager through its own `write_tags`
/// / `extract_tags` layer.
#[test]
fn multi_value_tlan_is_split_into_separate_items_by_lofty_itself() {
    let (_dir, path) = copy_fixture("silence.mp3");
    poke_raw_language_value(&path, "eng\u{0}swe");

    let read_back =
        extract_tags(&path).unwrap_or_else(|e| panic!("silence.mp3: extract_tags failed: {e}"));
    let values = read_back
        .get(TAG_LANGUAGE)
        .map(Vec::as_slice)
        .unwrap_or_default();
    assert_eq!(
        values,
        ["eng".to_string(), "swe".to_string()],
        "lofty must already have split the multi-value TLAN into separate items, in order, \
         with no embedded null character left in either one — got {values:?}"
    );

    // LANG-002's "the first is the primary language", applied to what
    // extract_tags actually hands back: take the first element.
    let primary = values.first().expect("must have at least one value");
    let stored = mm_core::metadata::language::parse_stored_language(primary);
    assert_eq!(stored.tag.tag, "en");
}

/// Issue #254 — pre-existing, not caused by this work and not specific to
/// `language`: a multi-value ID3 field does not survive a LATER, unrelated
/// save. `multi_value_tlan_is_split_into_separate_items_by_lofty_itself`
/// above proves the two values are both there immediately after being
/// written; this test proves that a completely unrelated `write_tags` call
/// afterwards (title only, `language` not even present in the map) still
/// silently loses the first of the two values. Marked `#[ignore]` so the
/// suite stays green until issue #254 is actually fixed — remove the
/// `#[ignore]` attribute then and this test starts checking the fix.
#[test]
#[ignore = "issue #254 — a multi-value ID3 field does not survive an unrelated save"]
fn multi_value_tlan_does_not_survive_an_unrelated_save() {
    let (_dir, path) = copy_fixture("silence.mp3");
    poke_raw_language_value(&path, "eng\u{0}swe");
    assert_eq!(
        extract_tags(&path)
            .unwrap()
            .get(TAG_LANGUAGE)
            .map(Vec::as_slice),
        Some(["eng".to_string(), "swe".to_string()].as_slice()),
        "sanity check: both values must be there immediately after writing"
    );

    let mut other_field = TagMap::new();
    other_field.insert(TAG_TITLE.to_string(), vec!["New Title".to_string()]);
    write_tags(&path, &other_field)
        .unwrap_or_else(|e| panic!("silence.mp3: write_tags failed: {e}"));

    assert_eq!(
        extract_tags(&path)
            .unwrap()
            .get(TAG_LANGUAGE)
            .map(Vec::as_slice),
        Some(["eng".to_string(), "swe".to_string()].as_slice()),
        "an unrelated save must not lose either value of a multi-value field it never touched"
    );
}

// ---------------------------------------------------------------------------
// The rule engine, against REAL files of every format (review item 2 of the
// second language-policy review round)
// ---------------------------------------------------------------------------

/// Review item 2, MUST FIX: an independent review's own mutation testing
/// found that disabling either half of the rule engine's language
/// standardisation (the FILE value in `evaluator.rs`, the RULE's own
/// configured value in `rule_engine/mod.rs`) still passed every test in
/// this crate — because there was no test anywhere that actually wrote a
/// language into a REAL file of each format and evaluated a real rule
/// against it. `rule_engine/mod.rs` and `evaluator.rs` each gained their
/// own fast, synthetic-`TagMap` unit tests for the underlying logic; this
/// integration test proves the same property end to end, through
/// `write_tags` and `extract_tags`, on all four formats this crate writes
/// `language` into.
#[test]
fn language_rule_conditions_match_across_every_container_format() {
    let fixtures: &[&str] = &["silence.mp3", "silence.flac", "silence.m4a", "silence.wav"];

    for fixture in fixtures {
        let (_dir, path) = copy_fixture(fixture);
        let mut tags = TagMap::new();
        tags.insert(TAG_LANGUAGE.to_string(), vec!["en".to_string()]);
        write_tags(&path, &tags).unwrap_or_else(|e| panic!("{fixture}: write_tags failed: {e}"));

        let read_back =
            extract_tags(&path).unwrap_or_else(|e| panic!("{fixture}: extract_tags failed: {e}"));
        let ctx = mm_core::rule_engine::EvalContext::new(&read_back);

        // A rule written against EITHER form must match every format,
        // including the ones (MP3, and WAV's embedded ID3v2 — see
        // `wav_write_tags_uses_embedded_id3v2_not_riff_info` above) that
        // actually store the three-letter "eng", not the short "en".
        for rule_value in ["en", "eng"] {
            let rule = mm_core::rule_engine::Rule {
                name: "test".to_string(),
                priority: 0,
                enabled: true,
                conditions: vec![mm_core::rule_engine::Condition {
                    field: "language".to_string(),
                    operator: mm_core::rule_engine::ConditionOp::Equals,
                    value: rule_value.to_string(),
                }],
                condition_mode: mm_core::rule_engine::ConditionMode::All,
                template: "Matched".to_string(),
                stop_on_match: false,
            };
            let result = mm_core::rule_engine::evaluate_rule(&rule, &ctx)
                .unwrap_or_else(|e| panic!("{fixture}/{rule_value}: evaluate_rule failed: {e}"));
            assert_eq!(
                result.as_deref(),
                Some("Matched"),
                "{fixture}: a rule for \"language Equals {rule_value}\" must match this file, \
                 whatever form it actually stores \"en\" in"
            );
        }

        // `<Language>` in a template must render the STANDARD form for
        // every one of these files — "en" — not whichever raw text that
        // format's own container happens to hold.
        let rendered = mm_core::rule_engine::evaluate_template("<Language>", &ctx)
            .unwrap_or_else(|e| panic!("{fixture}: evaluate_template failed: {e}"));
        assert_eq!(
            rendered, "en",
            "{fixture}: <Language> must render the standard form regardless of container"
        );
    }
}

// ---------------------------------------------------------------------------
// The "what was stored" note, checked against what real files really store
// (third independent review round)
// ---------------------------------------------------------------------------

/// Third review round, item 1 (MUST FIX): an ordinary old three-letter
/// code — `eng`, `fre`, `ger`, `deu` — was told "... is an old or grouped
/// form that is no longer used — it is replaced with the current code", and
/// `und` on an MP3 was told "an ID3 tag has no three-letter code for
/// \"und\"". Reproduced before the fix with the `meedya` binary built from
/// `e18fb18`: `meedya edit t.mp3 --set language=eng` printed the
/// "replaced with \"en\"" note, and an independent reader (mutagen) showed
/// the MP3's `TLAN` frame holding `eng` — exactly what was typed.
///
/// Each case asks for the note first (`preview_conversion_note`, the same
/// call `meedya edit` makes before writing anything), then really writes the
/// value and reads back what the container holds, bypassing
/// `extract_tags`' own logic — so the test proves the note is silent for a
/// value that is stored as typed, or stored as the same language in the
/// container's own spelling.
#[test]
fn no_note_for_an_ordinary_three_letter_code_on_real_files() {
    use lofty::tag::TagType;
    use mm_core::metadata::language::preview_conversion_note;

    // (fixture, container read back, typed, what that container stores)
    let cases: &[(&str, TagType, &str, &str)] = &[
        ("silence.mp3", TagType::Id3v2, "eng", "eng"),
        ("silence.mp3", TagType::Id3v2, "fre", "fra"), // ID3 takes the terminology form
        ("silence.mp3", TagType::Id3v2, "ger", "deu"), // likewise
        ("silence.mp3", TagType::Id3v2, "deu", "deu"),
        ("silence.mp3", TagType::Id3v2, "und", "und"),
        ("silence.mp3", TagType::Id3v2, "xxx", "und"), // ID3's own "not known" marker
        ("silence.flac", TagType::VorbisComments, "eng", "en"),
        ("silence.flac", TagType::VorbisComments, "fre", "fr"),
        ("silence.flac", TagType::VorbisComments, "ger", "de"),
        ("silence.flac", TagType::VorbisComments, "deu", "de"),
        ("silence.flac", TagType::VorbisComments, "und", "und"),
    ];

    for (fixture, container, typed, expected_stored) in cases {
        let (_dir, path) = copy_fixture(fixture);
        assert_eq!(
            preview_conversion_note(&path, typed),
            None,
            "{fixture}: `--set language={typed}` must carry no note"
        );

        write_tags(&path, &build_tags(&[(TAG_LANGUAGE, typed)]))
            .unwrap_or_else(|e| panic!("{fixture}/{typed}: write_tags failed: {e}"));
        assert_eq!(
            read_raw_language_from_tag_type(&path, *container).as_deref(),
            Some(*expected_stored),
            "{fixture}/{typed}: what the {container:?} container really holds"
        );
    }
}

/// The other half of item 1: the notes that were already right must stay
/// EXACTLY as they were (compared word for word with what the `e18fb18`
/// binary printed), and each must describe what the file then really
/// stores.
#[test]
fn notes_that_were_already_right_are_unchanged_and_match_what_is_stored() {
    use lofty::tag::TagType;
    use mm_core::metadata::language::preview_conversion_note;

    let i_klingon = "\"i-klingon\" is an old or grouped form that is no longer used — it is \
                     replaced with the current code, \"tlh\"";
    let sgn_br = "\"sgn-BR\" is an old or grouped form that is no longer used — it is replaced \
                  with the current code, \"bzs\"";
    let cases: Vec<(&str, TagType, &str, String, &str)> = vec![
        (
            "silence.mp3",
            TagType::Id3v2,
            "pt-BR",
            "an ID3 tag can only hold the three-letter language code, so it will lose the \
             region you typed — it will be stored there as \"por\""
                .to_string(),
            "por",
        ),
        (
            "silence.mp3",
            TagType::Id3v2,
            "i-klingon",
            i_klingon.to_string(),
            "tlh",
        ),
        (
            "silence.flac",
            TagType::VorbisComments,
            "i-klingon",
            i_klingon.to_string(),
            "tlh",
        ),
        (
            "silence.mp3",
            TagType::Id3v2,
            "sgn-BR",
            format!(
                "an ID3 tag has no three-letter code for \"bzs\" at all, so it will be stored \
                 there as not known; {sgn_br}"
            ),
            "und",
        ),
        (
            "silence.flac",
            TagType::VorbisComments,
            "sgn-BR",
            sgn_br.to_string(),
            "bzs",
        ),
    ];

    for (fixture, container, typed, expected_note, expected_stored) in cases {
        let (_dir, path) = copy_fixture(fixture);
        assert_eq!(
            preview_conversion_note(&path, typed).as_deref(),
            Some(expected_note.as_str()),
            "{fixture}: the note for `--set language={typed}` must be unchanged"
        );

        write_tags(&path, &build_tags(&[(TAG_LANGUAGE, typed)]))
            .unwrap_or_else(|e| panic!("{fixture}/{typed}: write_tags failed: {e}"));
        assert_eq!(
            read_raw_language_from_tag_type(&path, container).as_deref(),
            Some(expected_stored),
            "{fixture}/{typed}: the note says this is what is stored — check it is"
        );
    }
}

/// Fourth review round, item S2, on real files: an old three-letter code as
/// the first part of a longer tag (`eng-Latn`) used to be reported on an MP3
/// as "an ID3 tag has no three-letter code for \"eng\" at all" — false, as
/// `eng` is the code ID3 uses for English. What really happens (reproduced
/// with the binary built from `aa7a30d`, and read back with mutagen): the
/// MP3's ID3 tag stores `und`, "not known", because an old code is only
/// understood on its own. Each case asks for the note first, exactly as
/// `meedya edit` does, then writes the value and reads back what the ID3 tag
/// really holds. The FLAC keeps what was typed, and its note says so too,
/// with what to type instead.
#[test]
fn an_old_code_inside_a_longer_tag_is_explained_on_real_files() {
    use lofty::tag::TagType;
    use mm_core::metadata::language::preview_conversion_note;

    // (typed, the old code, what to type instead)
    let cases: &[(&str, &str, &str)] = &[
        ("eng-Latn", "eng", "en-Latn"),
        ("ger-1996", "ger", "de-1996"),
        ("deu-1996", "deu", "de-1996"),
        ("eng-x-foo", "eng", "en-x-foo"),
        ("eng-u-ca-gregory", "eng", "en-u-ca-gregory"),
        ("fre-Latn-CA", "fre", "fr-Latn-CA"),
        ("eng-US-x-foo", "eng", "en-US-x-foo"),
    ];

    for (typed, code, suggestion) in cases {
        let (_dir, mp3) = copy_fixture("silence.mp3");
        assert_eq!(
            preview_conversion_note(&mp3, typed),
            Some(format!(
                "\"{code}\" is an old code; inside a longer tag it is not recognised, so an ID3 \
                 tag will store it as not known — type \"{suggestion}\" instead"
            )),
            "silence.mp3: the note for `--set language={typed}`"
        );
        write_tags(&mp3, &build_tags(&[(TAG_LANGUAGE, typed)]))
            .unwrap_or_else(|e| panic!("{typed}: write_tags failed: {e}"));
        assert_eq!(
            read_raw_language_from_tag_type(&mp3, TagType::Id3v2).as_deref(),
            Some("und"),
            "{typed}: the note says the ID3 tag stores \"not known\" — check it does"
        );

        let (_dir2, flac) = copy_fixture("silence.flac");
        assert_eq!(
            preview_conversion_note(&flac, typed),
            Some(format!(
                "\"{code}\" is an old code; inside a longer tag it is not recognised, but it is \
                 kept exactly as typed — type \"{suggestion}\" instead"
            )),
            "silence.flac: the note for `--set language={typed}`"
        );
        write_tags(&flac, &build_tags(&[(TAG_LANGUAGE, typed)]))
            .unwrap_or_else(|e| panic!("{typed}: write_tags failed: {e}"));
        assert_eq!(
            read_raw_language_from_tag_type(&flac, TagType::VorbisComments).as_deref(),
            Some(*typed),
            "{typed}: the note says a FLAC keeps it exactly as typed — check it does"
        );
    }
}

/// Fourth review round, item S2, the other half: an extended-language part
/// (`bra` in `sgn-bra`) is lost on an ID3 tag, which stores only `sgn`. The
/// note used to say only that "bra" is not on the official list.
#[test]
fn a_lost_extended_language_part_is_named_on_a_real_mp3() {
    use lofty::tag::TagType;
    use mm_core::metadata::language::preview_conversion_note;

    let (_dir, mp3) = copy_fixture("silence.mp3");
    assert_eq!(
        preview_conversion_note(&mp3, "sgn-bra").as_deref(),
        Some(
            "an ID3 tag can only hold the three-letter language code, so it will lose the \
             extended-language part you typed — it will be stored there as \"sgn\"; \"bra\" is \
             not on the official list of language subtags"
        )
    );
    write_tags(&mp3, &build_tags(&[(TAG_LANGUAGE, "sgn-bra")])).unwrap();
    assert_eq!(
        read_raw_language_from_tag_type(&mp3, TagType::Id3v2).as_deref(),
        Some("sgn"),
        "the note says \"sgn\" is what is stored — check it is"
    );
}

// ---------------------------------------------------------------------------
// Tags that disagree about the language (third review round, item 3)
// ---------------------------------------------------------------------------

/// A WAV whose RIFF INFO chunk says "fre" and whose embedded ID3 tag says
/// "ger" (built byte by byte by `fixtures/make_language_fixtures.py`, not by
/// MeedyaManager, which would never write tags that disagree). Before this
/// round the German was invisible: `extract_tags` showed "fre", and
/// resending "fre" printed a plain "✓ Set" while the ID3 tag went on saying
/// German. The priority is kept (TRACK-070: the full tag is read, the
/// three-letter one is not); what changes is that the disagreement is said.
#[test]
fn a_disagreement_between_tags_is_reported_and_left_alone_by_a_resend() {
    use mm_core::metadata::language::{disagreement_note, preview_conversion_note};

    let (_dir, path) = copy_fixture("lang_riff_fre_id3_ger.wav");
    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::RiffInfo).as_deref(),
        Some("fre"),
        "fixture sanity check"
    );
    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::Id3v2).as_deref(),
        Some("ger"),
        "fixture sanity check"
    );

    // Still ONE value shown — the priority rule is unchanged.
    assert_eq!(
        extract_tags(&path)
            .unwrap()
            .get(TAG_LANGUAGE)
            .map(Vec::as_slice),
        Some(["fre".to_string()].as_slice())
    );

    // Someone LOOKING at the file is told.
    assert_eq!(
        disagreement_note(&path).as_deref(),
        Some(
            "this file's ID3 tag says \"ger\", which disagrees with \"fre\" — only \"fre\" is \
             shown, because the tag that can hold the full language code is read first"
        )
    );

    // Someone resending "fre" is told the ID3 tag is left alone...
    assert_eq!(
        preview_conversion_note(&path, "fre").as_deref(),
        Some(
            "this file's ID3 tag says \"ger\", which disagrees with \"fre\" and will be left \
             alone, because \"fre\" is already the file's language"
        )
    );

    // ...and it really is: the resend is still a no-change (COMPAT-030 —
    // every editing screen resends every field, so this must not change).
    write_tags(&path, &build_tags(&[(TAG_LANGUAGE, "fre")]))
        .unwrap_or_else(|e| panic!("resending the shown value must not fail: {e}"));
    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::Id3v2).as_deref(),
        Some("ger"),
        "a resend must leave the disagreeing ID3 tag exactly as it was"
    );
    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::RiffInfo).as_deref(),
        Some("fre")
    );
}

/// The same disagreement in a FLAC that starts with an ID3 tag (Vorbis
/// comment "eng", leading ID3 tag "ger"). MeedyaManager can READ this file;
/// it cannot yet SAVE it (a separate, older fault, tracked as its own
/// issue), so only reading is checked here.
#[test]
fn a_flac_whose_leading_id3_tag_disagrees_is_reported() {
    use mm_core::metadata::language::disagreement_note;

    let (_dir, path) = copy_fixture("lang_vorbis_eng_id3_ger.flac");
    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::Id3v2).as_deref(),
        Some("ger"),
        "fixture sanity check: lofty must see the leading ID3 tag at all"
    );
    assert_eq!(
        extract_tags(&path)
            .unwrap()
            .get(TAG_LANGUAGE)
            .map(Vec::as_slice),
        Some(["eng".to_string()].as_slice())
    );
    let note = disagreement_note(&path).expect("the ID3 tag's \"ger\" must not be hidden");
    assert!(
        note.starts_with("this file's ID3 tag says \"ger\", which disagrees with \"eng\""),
        "{note}"
    );
}

/// No false alarms: files whose tags say the same thing in each tag's own
/// spelling — exactly what `write_tags` itself produces — must carry no
/// note. `en-GB` beside `eng` (an ID3 tag cannot hold a region) and `yue`
/// beside `und` (there is no three-letter code for Cantonese) are the cases
/// a plain "different standard form" comparison would wrongly report.
#[test]
fn tags_that_agree_in_their_own_spellings_carry_no_note() {
    use mm_core::metadata::language::disagreement_note;

    for (value, riff, id3) in [
        ("en", "en", "eng"),
        ("fr", "fr", "fra"),
        ("en-GB", "en-GB", "eng"),
        ("yue", "yue", "und"),
    ] {
        let (_dir, path) = copy_fixture("riff_language.wav");
        write_tags(&path, &build_tags(&[(TAG_LANGUAGE, value)]))
            .unwrap_or_else(|e| panic!("{value}: write_tags failed: {e}"));
        assert_eq!(
            read_raw_language_from_tag_type(&path, lofty::tag::TagType::RiffInfo).as_deref(),
            Some(riff),
            "{value}: setup"
        );
        assert_eq!(
            read_raw_language_from_tag_type(&path, lofty::tag::TagType::Id3v2).as_deref(),
            Some(id3),
            "{value}: setup"
        );
        assert_eq!(
            disagreement_note(&path),
            None,
            "{value}: the two tags agree, so there is nothing to report"
        );
    }

    // The fixture that disagrees, with the RIFF chunk's "fre" and an ID3
    // tag spelling French the other way ("fra"), agrees too.
    let (_dir, path) = copy_fixture("lang_riff_fre_id3_ger.wav");
    write_tags(&path, &build_tags(&[(TAG_LANGUAGE, "fr")])).unwrap();
    assert_eq!(disagreement_note(&path), None, "after setting fr");
}

/// A value nothing recognises beside a real code: "English" (a word, not a
/// code) in the RIFF chunk and "eng" in the ID3 tag. They may well mean the
/// same thing, but MeedyaManager cannot know that without guessing
/// (LANG-003), so it is reported for a person to check (COMPAT-040).
#[test]
fn an_unrecognised_value_beside_a_code_is_reported_not_guessed_at() {
    use mm_core::metadata::language::disagreement_note;

    let (_dir, path) = copy_fixture("lang_riff_english_id3_eng.wav");
    let note = disagreement_note(&path).expect("the two do not provably agree");
    assert!(
        note.starts_with("this file's ID3 tag says \"eng\", which disagrees with \"English\""),
        "{note}"
    );
}

/// Fourth review round, item M6 (the reviewer's P6): the same word nothing
/// recognises — "English" — in BOTH tags is agreement, whatever language it
/// means, so nothing may be reported. The reviewer made every unrecognised
/// shown value agree with nothing at all, and every test still passed; this
/// file (built by `make_language_fixtures.py`) is the case that notices.
#[test]
fn the_same_unrecognised_word_in_both_tags_is_agreement() {
    use mm_core::metadata::language::disagreement_note;

    let (_dir, path) = copy_fixture("lang_riff_english_id3_english.wav");
    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::RiffInfo).as_deref(),
        Some("English"),
        "fixture sanity check"
    );
    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::Id3v2).as_deref(),
        Some("English"),
        "fixture sanity check"
    );
    assert_eq!(
        extract_tags(&path)
            .unwrap()
            .get(TAG_LANGUAGE)
            .map(Vec::as_slice),
        Some(["English".to_string()].as_slice())
    );
    assert_eq!(
        disagreement_note(&path),
        None,
        "both tags say exactly the same thing"
    );
}

/// Fourth review round, item M6 (the reviewer's P5): an ID3 tag holding the
/// same disagreeing value twice ("ger", "ger" — two values, as ID3 version
/// 2.4 allows) must be named once. The reviewer removed the "each once"
/// check, and every test still passed; the note then read "says \"ger\"
/// and \"ger\", which disagree".
#[test]
fn the_same_hidden_value_twice_is_named_once() {
    use mm_core::metadata::language::disagreement_note;

    let (_dir, path) = copy_fixture("lang_riff_en_id3_ger_twice.wav");
    assert_eq!(
        read_raw_language_values_from_tag_type(&path, lofty::tag::TagType::Id3v2),
        vec!["ger".to_string(), "ger".to_string()],
        "fixture sanity check: the ID3 tag really holds \"ger\" twice"
    );
    assert_eq!(
        disagreement_note(&path).as_deref(),
        Some(
            "this file's ID3 tag says \"ger\", which disagrees with \"en\" — only \"en\" is \
             shown, because the tag that can hold the full language code is read first"
        )
    );
}

// ---------------------------------------------------------------------------
// Rule conditions on `language`, on real files (third review round, items 6-7)
// ---------------------------------------------------------------------------

/// Evaluate one `language` condition against `tags`, through the public
/// `evaluate_rule` — the same way a rename rule is checked.
fn language_rule_matches(
    tags: &TagMap,
    path_mode: bool,
    operator: mm_core::rule_engine::ConditionOp,
    value: &str,
) -> bool {
    let rule = mm_core::rule_engine::Rule {
        name: "test".to_string(),
        priority: 0,
        enabled: true,
        conditions: vec![mm_core::rule_engine::Condition {
            field: "language".to_string(),
            operator,
            value: value.to_string(),
        }],
        condition_mode: mm_core::rule_engine::ConditionMode::All,
        template: "Matched".to_string(),
        stop_on_match: false,
    };
    let ctx = mm_core::rule_engine::EvalContext::new(tags).with_path_mode(path_mode);
    mm_core::rule_engine::evaluate_rule(&rule, &ctx)
        .unwrap_or_else(|e| panic!("{operator:?} {value:?}: {e}"))
        .is_some()
}

/// Item 6, on a real MP3 whose `TLAN` frame holds two languages, "eng" and
/// "fra": `Matches` used to try "eng; fra" — the two run together — so the
/// unanchored pattern "fra" matched while the file's path was being built
/// from its FIRST language (English), and an anchored pattern for either
/// language never matched.
#[test]
fn language_matches_on_a_real_file_with_two_languages() {
    use mm_core::rule_engine::ConditionOp::Matches;

    let (_dir, path) = copy_fixture("silence.mp3");
    poke_raw_language_value(&path, "eng\u{0}fra");
    let tags = extract_tags(&path).unwrap();
    assert_eq!(
        tags.get(TAG_LANGUAGE).map(Vec::as_slice),
        Some(["eng".to_string(), "fra".to_string()].as_slice()),
        "fixture sanity check: two separate values"
    );

    // Building a path: the first stored language only.
    assert!(!language_rule_matches(&tags, true, Matches, "fra"));
    assert!(language_rule_matches(&tags, true, Matches, "^eng$"));
    // Otherwise: each stored language on its own.
    assert!(language_rule_matches(&tags, false, Matches, "^fra$"));
    assert!(!language_rule_matches(&tags, false, Matches, "^eng; fra$"));
}

/// Item 7, on a real MP3 given "en" (so it stores "eng"): `Contains`,
/// `StartsWith`, `EndsWith` and `NotContains` are tried against both the
/// standard form ("en") and the stored text ("eng").
#[test]
fn language_text_conditions_on_a_real_file_try_both_forms() {
    use mm_core::rule_engine::ConditionOp::{Contains, EndsWith, NotContains, StartsWith};

    let (_dir, path) = copy_fixture("silence.mp3");
    write_tags(&path, &build_tags(&[(TAG_LANGUAGE, "en")])).unwrap();
    let tags = extract_tags(&path).unwrap();
    assert_eq!(
        tags.get(TAG_LANGUAGE).map(Vec::as_slice),
        Some(["eng".to_string()].as_slice()),
        "an MP3 stores the three-letter code"
    );

    assert!(language_rule_matches(&tags, true, Contains, "ng"));
    assert!(language_rule_matches(&tags, true, EndsWith, "ng"));
    assert!(language_rule_matches(&tags, true, StartsWith, "eng"));
    assert!(language_rule_matches(&tags, true, Contains, "en"));
    assert!(!language_rule_matches(&tags, true, NotContains, "ng"));
    assert!(language_rule_matches(&tags, true, NotContains, "fr"));
}

// ---------------------------------------------------------------------------
// Code the third reviewer could break without any test failing (item 5)
// ---------------------------------------------------------------------------
//
// The third review round deliberately broke each new piece of code from the
// second round, one at a time, and ran every test. Eight breakages still
// passed; the lead asked for a test that fails for each. The unit-level ones
// (the lost-part names and "and" joining) are in `language.rs`; these need a
// real file.

/// Breakage N2: reading a file's languages removed repeats by their raw
/// text instead of their standard form, so a FLAC whose Vorbis comments
/// hold both "en" and "eng" (the same language, written twice by two
/// tools) read back as two languages. It must read back as one — the
/// first text seen is kept, as editors show stored text (COMPAT-040).
#[test]
fn one_language_written_two_ways_in_the_same_tag_reads_back_once() {
    let (_dir, path) = copy_fixture("silence.flac");
    poke_raw_language_values(&path, &["en", "eng"]);
    assert_eq!(
        read_raw_language_values_from_tag_type(&path, lofty::tag::TagType::VorbisComments),
        vec!["en".to_string(), "eng".to_string()],
        "fixture sanity check: both comments are really in the file"
    );

    assert_eq!(
        extract_tags(&path)
            .unwrap()
            .get(TAG_LANGUAGE)
            .map(Vec::as_slice),
        Some(["en".to_string()].as_slice()),
        "\"en\" and \"eng\" are one language, told twice"
    );

    // Two genuinely different languages are both kept.
    let (_dir2, path2) = copy_fixture("silence.flac");
    poke_raw_language_values(&path2, &["en", "fr"]);
    assert_eq!(
        extract_tags(&path2)
            .unwrap()
            .get(TAG_LANGUAGE)
            .map(Vec::as_slice),
        Some(["en".to_string(), "fr".to_string()].as_slice())
    );
}

/// Breakage N4: the preview skipped its "is this value already the file's
/// language?" check, so an unchanged value was described as if it were
/// being written afresh. An MP3 whose ID3 tag already holds "pt-BR"
/// (written by another tool — MeedyaManager would store "por") resends
/// "pt-BR": nothing will be written, so there is no region to lose and
/// nothing to say.
#[test]
fn resending_a_value_already_stored_gets_no_conversion_note() {
    use mm_core::metadata::language::preview_conversion_note;

    let (_dir, path) = copy_fixture("silence.mp3");
    poke_raw_language_value(&path, "pt-BR");
    assert_eq!(
        preview_conversion_note(&path, "pt-BR"),
        None,
        "pt-BR is already there; nothing is converted, so nothing is lost"
    );
    // The same value typed on a file that does NOT already hold it does
    // get the note — so the silence above is the check, not an accident.
    let (_dir2, fresh) = copy_fixture("silence.mp3");
    assert!(preview_conversion_note(&fresh, "pt-BR").is_some());
}

/// Breakage N5: the preview looked only at the file's main tag, not every
/// tag `write_tags` will really change. A FLAC's main tag is its Vorbis
/// comments, which keep "pt-BR" whole — but the FLAC below also starts with
/// an ID3 tag that already holds a language, so `write_tags` changes that
/// one too, and an ID3 tag can only keep "por". The note must say so.
#[test]
fn the_preview_covers_every_tag_that_will_change_not_just_the_main_one() {
    use mm_core::metadata::language::preview_conversion_note;

    let (_dir, path) = copy_fixture("lang_vorbis_eng_id3_ger.flac");
    let note = preview_conversion_note(&path, "pt-BR")
        .expect("the leading ID3 tag loses the region, even though the Vorbis comment keeps it");
    assert!(note.contains("an ID3 tag"), "{note}");
    assert!(note.contains("\"por\""), "{note}");
}

/// Breakage N6: when checking whether a resent language is unchanged,
/// `write_tags` compared it with EVERY tag's value run together, rather
/// than with what `extract_tags` shows. The desktop apps read every field,
/// let a person change one, and send every field back — so on a WAV whose
/// RIFF INFO chunk holds "English" (a word, not a code) and whose ID3 tag
/// holds "eng", the shown "English" no longer matched "English; eng", was
/// treated as a new value, and the WHOLE save was refused because
/// "English" is not a language code. A title-only change must succeed, and
/// the language must be left exactly as it was.
#[test]
fn a_program_that_writes_back_every_field_can_still_change_only_the_title() {
    let (_dir, path) = copy_fixture("lang_riff_english_id3_eng.wav");

    // Read everything, change only the title, write everything back —
    // exactly what every editing screen in this project does on Save.
    let mut everything = extract_tags(&path).unwrap();
    assert_eq!(
        everything.get(TAG_LANGUAGE).map(Vec::as_slice),
        Some(["English".to_string()].as_slice()),
        "fixture sanity check: the full tag's text is what is shown"
    );
    everything.insert(TAG_TITLE.to_string(), vec!["New".to_string()]);
    write_tags(&path, &everything)
        .unwrap_or_else(|e| panic!("writing back unchanged fields must not be refused: {e}"));

    // The new title went into the file's main (ID3) tag. The RIFF INFO
    // chunk's own "Old" title is still there too, so both come back — that
    // is issue #255 (any field, not only language, goes stale in the tag
    // that was not written), not what this test is about.
    let after = extract_tags(&path).unwrap();
    assert!(
        after
            .get(TAG_TITLE)
            .is_some_and(|titles| titles.contains(&"New".to_string())),
        "the title change must have been made: {:?}",
        after.get(TAG_TITLE)
    );
    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::RiffInfo).as_deref(),
        Some("English"),
        "the RIFF INFO language must be untouched"
    );
    assert_eq!(
        read_raw_language_from_tag_type(&path, lofty::tag::TagType::Id3v2).as_deref(),
        Some("eng"),
        "the ID3 language must be untouched"
    );
}

/// Like `poke_raw_language_value`, but puts SEVERAL separate language
/// items into the file's main tag — for a Vorbis comment, one `LANGUAGE=`
/// line each, which is how two tools that each added a value leave it.
fn poke_raw_language_values(path: &Path, values: &[&str]) {
    use lofty::config::WriteOptions;
    use lofty::file::TaggedFileExt;
    use lofty::probe::Probe;
    use lofty::tag::{ItemKey, ItemValue, Tag, TagExt, TagItem};

    let mut tagged_file = Probe::open(path)
        .unwrap_or_else(|e| panic!("{}: cannot probe: {e}", path.display()))
        .read()
        .unwrap_or_else(|e| panic!("{}: cannot read: {e}", path.display()));
    if tagged_file.primary_tag_mut().is_none() {
        let tag_type = tagged_file.primary_tag_type();
        tagged_file.insert_tag(Tag::new(tag_type));
    }
    let tag = tagged_file
        .primary_tag_mut()
        .expect("primary tag must exist after insert_tag");
    tag.remove_key(&ItemKey::Language);
    for value in values {
        tag.push(TagItem::new(
            ItemKey::Language,
            ItemValue::Text((*value).to_string()),
        ));
    }
    tag.save_to_path(path, WriteOptions::default())
        .unwrap_or_else(|e| panic!("{}: cannot save: {e}", path.display()));
}

/// Every language value ONE kind of tag holds, in order — the several-value
/// version of `read_raw_language_from_tag_type`.
fn read_raw_language_values_from_tag_type(
    path: &Path,
    tag_type: lofty::tag::TagType,
) -> Vec<String> {
    use lofty::file::TaggedFileExt;
    use lofty::probe::Probe;
    use lofty::tag::{ItemKey, ItemValue};

    let tagged_file = Probe::open(path)
        .unwrap_or_else(|e| panic!("{}: cannot probe: {e}", path.display()))
        .read()
        .unwrap_or_else(|e| panic!("{}: cannot read: {e}", path.display()));
    let mut values = Vec::new();
    for tag in tagged_file.tags() {
        if tag.tag_type() == tag_type {
            for item in tag.get_items(&ItemKey::Language) {
                if let ItemValue::Text(text) = item.value() {
                    values.push(text.clone());
                }
            }
        }
    }
    values
}

// ---------------------------------------------------------------------------
// Cover art round trip — raw and integrity-guarded
// ---------------------------------------------------------------------------

#[test]
fn cover_art_round_trip() {
    let _guard = ConfigDirGuard::new();

    let cover = fs::read(fixtures_dir().join("cover.png"))
        .expect("cover.png fixture must be readable — see tests/fixtures/README.md");

    // === Raw metadata layer: embed_cover_art / remove_cover_art ============
    {
        let (_dir, path) = copy_fixture("silence.mp3");

        embed_cover_art(&path, &cover, "image/png")
            .unwrap_or_else(|e| panic!("embed_cover_art failed: {e}"));
        let extracted = extract_cover_art(&path)
            .unwrap_or_else(|e| panic!("extract_cover_art failed: {e}"))
            .expect("cover art must be present immediately after embedding");
        assert_eq!(
            extracted.data, cover,
            "embedded cover art bytes must round-trip exactly"
        );
        assert_eq!(extracted.mime, "image/png");

        remove_cover_art(&path).unwrap_or_else(|e| panic!("remove_cover_art failed: {e}"));
        let after_removal = extract_cover_art(&path)
            .unwrap_or_else(|e| panic!("extract_cover_art failed after removal: {e}"));
        assert!(
            after_removal.is_none(),
            "cover art must be gone after remove_cover_art, got {after_removal:?}"
        );
    }

    // === Integrity-guarded layer: embed_cover_art_safe / remove_cover_art_safe
    {
        let (_dir, path) = copy_fixture("silence.mp3");

        let embed_result = embed_cover_art_safe(&path, &cover, "image/png");
        assert!(
            embed_result.success,
            "embed_cover_art_safe failed: {:?}",
            embed_result.error
        );
        let extracted = extract_cover_art(&path)
            .unwrap_or_else(|e| panic!("extract_cover_art failed: {e}"))
            .expect("cover art must be present after embed_cover_art_safe");
        assert_eq!(extracted.data, cover);
        assert_eq!(extracted.mime, "image/png");

        let remove_result = remove_cover_art_safe(&path);
        assert!(
            remove_result.success,
            "remove_cover_art_safe failed: {:?}",
            remove_result.error
        );
        let after_removal = extract_cover_art(&path)
            .unwrap_or_else(|e| panic!("extract_cover_art failed after removal: {e}"));
        assert!(
            after_removal.is_none(),
            "cover art must be gone after remove_cover_art_safe, got {after_removal:?}"
        );
    }
}
