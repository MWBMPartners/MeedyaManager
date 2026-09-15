// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — shared helpers for the `mm-cli` unit tests.
//
// Why this file exists
// --------------------
// Several command modules need to point the configuration directory at a
// private temporary folder for the duration of one test. They do that by
// setting the `MM_CONFIG_DIR` environment variable, and an environment
// variable is shared by every thread in the test binary — `cargo test` runs
// tests in parallel threads, not separate processes.
//
// That means there can only ever be **one** lock guarding that variable. If
// two modules each kept their own private lock, two tests could hold "their"
// lock at the same time and still be fighting over the same variable: one
// test would see the other test's temporary directory and fail for reasons
// that have nothing to do with what it is testing. `edit.rs` carried exactly
// that warning in a comment; `scan.rs` now needs the same guard, so the lock
// and the guard live here, once, and every module borrows them.
//
// The whole module is compiled only for tests.
//
// `unsafe` is denied across this crate. It is allowed here, and only here,
// because changing an environment variable is `unsafe` in Rust 2024: another
// thread could be reading it at the same moment. The lock below is exactly
// the discipline that marker asks for, and keeping the two together in one
// small file is why this module exists.
#![allow(unsafe_code)]

/// The one and only lock protecting `MM_CONFIG_DIR` inside this test binary.
///
/// Anything that reads or writes that variable must hold this lock first.
pub(crate) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Points `MM_CONFIG_DIR` at a private temporary folder for the lifetime of
/// one test, and puts the environment back the way it was when the test ends.
///
/// Restoring the variable when the guard is dropped — rather than with a line
/// at the end of the test — matters: a failed assertion ends the test by
/// panicking, which skips any remaining lines. Without this, one failing test
/// would leak its temporary configuration directory (and, worse, a live Test
/// Mode manifest) into every test that ran after it.
pub(crate) struct ConfigDirGuard {
    // Fields are dropped top to bottom, so the temporary folder is deleted
    // before the lock is handed to the next test that wants it.
    _dir: tempfile::TempDir,
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl ConfigDirGuard {
    /// Take the lock, make a fresh temporary folder, and point the
    /// configuration directory at it.
    pub(crate) fn new() -> Self {
        // A test that panicked while holding the lock leaves it "poisoned".
        // That only tells us a previous test failed — it says nothing about
        // whether the data behind the lock is damaged, because there is no
        // data behind it. So we take the lock anyway rather than turning one
        // failure into a cascade of unrelated ones.
        let lock = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = tempfile::tempdir().unwrap();
        // SAFETY: changing an environment variable is unsafe in Rust 2024
        // because another thread could be reading it at the same moment. The
        // lock taken immediately above is what makes that impossible here.
        unsafe {
            std::env::set_var("MM_CONFIG_DIR", dir.path());
        }
        Self {
            _dir: dir,
            _lock: lock,
        }
    }
}

impl Drop for ConfigDirGuard {
    fn drop(&mut self) {
        // SAFETY: as above — this guard still holds the lock at this point,
        // so no other test thread can be reading the variable.
        unsafe {
            std::env::remove_var("MM_CONFIG_DIR");
        }
    }
}

/// Write a small but genuinely valid WAV file to `path`.
///
/// It holds a tenth of a second of silence: 8 kHz, one channel, 16 bits per
/// sample. The silence is not decoration — the `lofty` tagging library
/// rejects a bare 44-byte header with no audio behind it, so a stub file
/// would never reach the code these tests are actually trying to exercise.
///
/// Any missing parent folders are created first, so a test can ask for a
/// nested path without setting the folders up itself.
pub(crate) fn write_wav_fixture(path: &std::path::Path) {
    const SAMPLE_RATE: u32 = 8_000;
    const CHANNELS: u16 = 1;
    const BITS_PER_SAMPLE: u16 = 16;
    const FRAMES: u32 = 800; // 800 frames at 8 kHz = 0.1 seconds

    // Bytes taken up by one moment of sound across all channels.
    let block_align = CHANNELS * BITS_PER_SAMPLE / 8;
    // Bytes of audio per second — a field the WAV header must carry.
    let byte_rate = SAMPLE_RATE * u32::from(block_align);
    // Total bytes of actual audio: 1,600, for 1,644 bytes of file.
    let data_len = FRAMES * u32::from(block_align);

    let mut wav: Vec<u8> = Vec::with_capacity(44 + data_len as usize);

    // ── RIFF container header ───────────────────────────────────────────
    wav.extend_from_slice(b"RIFF");
    // Everything after this field: 4 ("WAVE") + 24 (fmt chunk) + 8 + audio.
    wav.extend_from_slice(&(4 + 24 + 8 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    // ── "fmt " chunk — how to interpret the audio (16-byte PCM form) ────
    wav.extend_from_slice(b"fmt "); // note the trailing space: the name is 4 characters
    wav.extend_from_slice(&16u32.to_le_bytes()); // this chunk is 16 bytes long
    wav.extend_from_slice(&1u16.to_le_bytes()); // format 1 means uncompressed PCM
    wav.extend_from_slice(&CHANNELS.to_le_bytes());
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&BITS_PER_SAMPLE.to_le_bytes());

    // ── "data" chunk — real silence, not an empty stub ───────────────────
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend(std::iter::repeat_n(0u8, data_len as usize));

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, &wav).expect("WAV fixture must be writable");
}

/// Write a WAV fixture and stamp artist / album / title onto it.
///
/// Shared by the `scan` and `watch` tests: both need a file whose embedded
/// tags actually drive the `<Artist>/<Album>/<Title>` template, and a file
/// with no tags would silently produce an empty destination path instead of
/// proving anything.
pub(crate) fn write_tagged_wav(path: &std::path::Path, artist: &str, album: &str, title: &str) {
    write_wav_fixture(path);

    let mut tags = mm_core::metadata::TagMap::new();
    tags.insert("artist".to_string(), vec![artist.to_string()]);
    tags.insert("album".to_string(), vec![album.to_string()]);
    tags.insert("title".to_string(), vec![title.to_string()]);

    mm_core::metadata::write_tags(path, &tags).unwrap();
}
