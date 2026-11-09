// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — crates/mm-cli/tests/debug_language_warning.rs
//
// Runs the real `meedya` program and reads what `meedya debug` prints for a
// person (not `--json`), to prove the language warning is really there.
//
// WHY THIS IS A SEPARATE TEST FILE THAT RUNS THE PROGRAM
// -------------------------------------------------------
// `meedya debug` prints its human-readable output straight to the screen,
// one line at a time, so a test inside the program cannot see it. The
// fourth independent review of the language-policy work (issue #251, item
// M6, the reviewer's fault "P7") switched that warning line off — the line
// that tells a person the file's tags disagree about the language — and
// every test still passed, because each test only checked the note was
// WORKED OUT, never that it was PRINTED. Cargo builds the program before
// running a test file in this folder and says where it is
// (`CARGO_BIN_EXE_meedya`), so this runs exactly what a person would run.
//
// WHAT IT CANNOT DO
// -----------------
// It checks the words, not the colours: colour is switched off (`NO_COLOR`)
// so the text can be compared exactly.
//
// License: GPL-2.0-or-later

use std::path::{Path, PathBuf};
use std::process::Command;

/// Copy one of `mm-core`'s committed test files into `dir` and return the
/// copy's path — the committed file itself is never touched.
fn copy_core_fixture(name: &str, dir: &Path) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../mm-core/tests/fixtures")
        .join(name);
    let copy = dir.join(name);
    std::fs::copy(&source, &copy)
        .unwrap_or_else(|e| panic!("cannot copy test file {}: {e}", source.display()));
    copy
}

/// Run `meedya debug <file>` with a private, empty settings folder (so the
/// developer's own settings are never read or changed) and colour off, and
/// return everything it printed to standard output.
fn debug_output(file: &Path, config_dir: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_meedya"))
        .arg("debug")
        .arg(file)
        .env("MM_CONFIG_DIR", config_dir)
        .env("NO_COLOR", "1")
        .output()
        .expect("the meedya program must start");
    assert!(
        output.status.success(),
        "meedya debug failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("the output is text")
}

/// A WAV whose RIFF INFO chunk says "fre" and whose ID3 tag says "ger": the
/// warning must be printed, as its own line, after the tags table and
/// before the next section.
#[test]
fn meedya_debug_prints_the_language_warning_for_a_person() {
    let files = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let file = copy_core_fixture("lang_riff_fre_id3_ger.wav", files.path());

    let stdout = debug_output(&file, config.path());
    let lines: Vec<&str> = stdout.lines().collect();
    let warning = "⚠ this file's ID3 tag says \"ger\", which disagrees with \"fre\" — only \"fre\" \
                   is shown, because the tag that can hold the full language code is read first";
    let at = lines
        .iter()
        .position(|line| *line == warning)
        .unwrap_or_else(|| panic!("the warning line is missing:\n{stdout}"));

    let tags_heading = lines
        .iter()
        .position(|line| *line == "Metadata Tags")
        .unwrap_or_else(|| panic!("no tags heading:\n{stdout}"));
    let next_heading = lines
        .iter()
        .position(|line| *line == "Cover Art")
        .unwrap_or_else(|| panic!("no cover art heading:\n{stdout}"));
    assert!(
        tags_heading < at && at < next_heading,
        "the warning belongs under the tags table it explains:\n{stdout}"
    );
}

/// The other side: a file whose tags agree prints no such line at all.
#[test]
fn meedya_debug_prints_no_language_warning_when_the_tags_agree() {
    let files = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let file = copy_core_fixture("riff_language.wav", files.path());

    let stdout = debug_output(&file, config.path());
    assert!(
        !stdout.contains("ID3 tag says"),
        "no warning when nothing disagrees:\n{stdout}"
    );
}
