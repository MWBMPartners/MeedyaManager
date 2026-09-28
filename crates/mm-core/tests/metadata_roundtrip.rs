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
fn wav_riff_info_round_trip() {
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
/// the reason explained at length on `wav_riff_info_round_trip` above — a
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
