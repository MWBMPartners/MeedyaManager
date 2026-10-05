// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — crates/mm-core/src/metadata/riff_info.rs
//
// A small, bounded reader for a WAV file's RIFF INFO list, used only to make
// sure no save ever loses anything else from that list.
//
// WHY THIS EXISTS (Codex's catch-up review of the language-policy branch,
// finding 1, issue #251; widened by the stand-in review of round 6)
// ---------------------------------------------------------------------------
// A WAV file can carry two tag sections at once: an embedded ID3 tag, and the
// older RIFF INFO list (four-letter entries such as `INAM`, the title, and
// `ILNG`, the language). Whenever a save rewrites that list — keeping its
// language in step with a new one, or removing a field from it — the `lofty`
// tag library writes the WHOLE list back from what it read.
//
// RIFF INFO names no text encoding. Older Windows tools write the computer's
// own code page, so a title such as "Café" can be the Latin-1 bytes
// `43 61 66 E9`, which are not valid UTF-8. `lofty` cannot read such an entry,
// so — in its default, forgiving reading mode — it quietly leaves it out of
// what it read. Writing the list back then leaves it out of the file too. So
// changing the language deleted the title, and the save reported success.
// Reproduced with the `meedya` binary built from `a150926` before this file
// existed: see the commit that added it. Round 6 guarded language saves only;
// the binary built from `49cec29` still deleted that title on
// `--remove artist` (the file had no artist at all) and on `--remove-cover`
// (a RIFF INFO list cannot even hold a picture), because both rewrote every
// tag section the file had, whether or not it held what was being removed.
//
// So, around EVERY save of a WAV, this module reads the list's entries RAW —
// four-letter id and exact bytes, with no decoding at all, so nothing can be
// skipped — before and after, and `check_entries_kept` compares them: every
// entry other than the ones the save was asked to change must be byte-for-byte
// the same and in the same order, and each asked-for entry must hold exactly
// what was asked (or be gone, when it was removed). Anything else is refused,
// naming what would be lost.
//
// WHAT THIS CANNOT DO
// -------------------
// * It only guards the RIFF INFO list. It knows nothing about the embedded ID3
//   tag or any other section of the file.
// * It refuses every difference but one, including harmless ones: an entry
//   another tool wrote with two zero bytes at its end instead of one would be
//   rewritten by `lofty` with one, and the save is refused. That is
//   deliberate — it cannot tell a harmless difference from a harmful one
//   without guessing, and a refusal can be retried by a person; lost data
//   cannot be got back. The one exception (the stand-in review of round 6,
//   L2): an entry with no zero byte at its end, which `lofty` rewrites with
//   one (and the padding byte that may then follow). Text ends at the first
//   zero byte either way, so nothing a person could read is changed — and
//   without the exception every such file could never be saved at all.
// * It is a reader, not a validator of the whole WAV format: it walks the
//   file's top-level chunks only as far as it needs to find `LIST INFO`.
//
// License: GPL-2.0-or-later

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// The four-letter id of the RIFF INFO language entry.
pub(crate) const LANGUAGE_ID: [u8; 4] = *b"ILNG";

/// At most this many top-level chunks are looked at before giving up. A real
/// WAV has a handful (`fmt `, `data`, `LIST`, perhaps `ID3 `); the limit only
/// exists so a damaged file whose chunk sizes are all zero cannot keep the
/// reader walking for ever.
const MAX_CHUNKS: usize = 10_000;

/// An INFO list larger than this is not read. Real ones are a few hundred
/// bytes; refusing a save because a list is enormous is better than reading a
/// gigabyte into memory to check it.
const MAX_INFO_LIST_BYTES: u64 = 16 * 1024 * 1024;

/// One RIFF INFO entry, exactly as stored: its four-letter id, and its bytes
/// as counted by its own size field (so a zero byte at the end, when the
/// writer counted one, is included; the padding byte after an odd-sized
/// entry, which the size does not count, is not).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InfoEntry {
    /// The four-letter id, such as `INAM` or `ILNG`.
    pub(crate) id: [u8; 4],
    /// The entry's bytes, undecoded.
    pub(crate) data: Vec<u8>,
}

impl InfoEntry {
    /// The id as text, for messages (`INAM`). An id that is not plain text
    /// shows its bytes instead, so a message can never be garbled by it.
    pub(crate) fn id_text(&self) -> String {
        if self.id.iter().all(u8::is_ascii_graphic) {
            String::from_utf8_lossy(&self.id).into_owned()
        } else {
            format!("{:02x?}", self.id)
        }
    }
}

/// What a raw read of a WAV file's RIFF INFO lists found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InfoLists {
    /// Every entry, in file order, across every `LIST INFO` chunk.
    pub(crate) entries: Vec<InfoEntry>,
    /// How many `LIST INFO` chunks the file has. A file should have one at
    /// most; the stand-in review of round 6 (M1, issue #259) is why the
    /// count is kept: `lofty` rewrites only the FIRST list, copying the
    /// second's entries into it, so a save that rewrites the list of such a
    /// file must be refused before anything is written — the comparison of
    /// entries alone could not tell, because the tag library's predicted
    /// list already holds the second list's entries.
    pub(crate) list_count: usize,
}

/// Every RIFF INFO entry in the WAV file at `path`, in file order, across
/// every `LIST` chunk of type `INFO` (a file should have one; if it has more,
/// all are read, in order), and how many such chunks there are.
///
/// Returns `Ok(None)` when the file is not a RIFF `WAVE` file at all, and
/// no entries and a count of 0 for a WAV with no INFO list.
///
/// # Errors
/// A plain-English description when the file cannot be read or its chunk
/// sizes do not fit inside it — the caller refuses the save rather than
/// guessing what the list holds.
pub(crate) fn read_info_lists(path: &Path) -> Result<Option<InfoLists>, String> {
    let describe = |e: std::io::Error| format!("its RIFF INFO list could not be read: {e}");
    let mut file = File::open(path).map_err(describe)?;
    let file_len = file.metadata().map_err(describe)?.len();

    let mut header = [0u8; 12];
    if file_len < 12 {
        return Ok(None);
    }
    file.read_exact(&mut header).map_err(describe)?;
    if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
        return Ok(None);
    }

    let mut entries = Vec::new();
    let mut list_count = 0usize;
    let mut pos: u64 = 12;
    let mut chunks_seen = 0usize;
    while pos + 8 <= file_len {
        chunks_seen += 1;
        if chunks_seen > MAX_CHUNKS {
            return Err(format!(
                "its RIFF INFO list could not be read: the file has more than {MAX_CHUNKS} \
                 sections, which no real WAV file has"
            ));
        }
        file.seek(SeekFrom::Start(pos)).map_err(describe)?;
        let mut chunk_header = [0u8; 8];
        file.read_exact(&mut chunk_header).map_err(describe)?;
        let size = u64::from(u32::from_le_bytes([
            chunk_header[4],
            chunk_header[5],
            chunk_header[6],
            chunk_header[7],
        ]));
        let body_start = pos + 8;
        if body_start + size > file_len {
            // Many tools write a last chunk whose size runs past the end of
            // the file (a recording cut short). That is only a problem here
            // when it is the INFO list itself.
            if &chunk_header[0..4] == b"LIST" && is_info_list(&mut file, body_start, file_len)? {
                return Err(
                    "its RIFF INFO list could not be read: the list says it is longer than the \
                     file"
                        .to_string(),
                );
            }
            break;
        }
        if &chunk_header[0..4] == b"LIST"
            && size >= 4
            && is_info_list(&mut file, body_start, file_len)?
        {
            if size > MAX_INFO_LIST_BYTES {
                return Err(format!(
                    "its RIFF INFO list could not be read: it is larger than {} MB",
                    MAX_INFO_LIST_BYTES / (1024 * 1024)
                ));
            }
            let mut payload = vec![
                0u8;
                usize::try_from(size).map_err(|_| {
                    "its RIFF INFO list could not be read: it is too large".to_string()
                })?
            ];
            file.seek(SeekFrom::Start(body_start)).map_err(describe)?;
            file.read_exact(&mut payload).map_err(describe)?;
            entries.extend(parse_info_payload(&payload)?);
            list_count += 1;
        }
        // A chunk with an odd size is followed by one padding byte its size
        // does not count.
        pos = body_start + size + (size & 1);
    }
    Ok(Some(InfoLists {
        entries,
        list_count,
    }))
}

/// Whether the `LIST` chunk whose body starts at `body_start` is of type
/// `INFO` (its first four bytes).
fn is_info_list(file: &mut File, body_start: u64, file_len: u64) -> Result<bool, String> {
    if body_start + 4 > file_len {
        return Ok(false);
    }
    let describe = |e: std::io::Error| format!("its RIFF INFO list could not be read: {e}");
    file.seek(SeekFrom::Start(body_start)).map_err(describe)?;
    let mut list_type = [0u8; 4];
    file.read_exact(&mut list_type).map_err(describe)?;
    Ok(&list_type == b"INFO")
}

/// The entries of one `LIST INFO` chunk's body (starting with the four bytes
/// `INFO`).
///
/// # Errors
/// When an entry's size runs past the end of the list.
fn parse_info_payload(payload: &[u8]) -> Result<Vec<InfoEntry>, String> {
    let mut entries = Vec::new();
    let mut p = 4usize; // skip "INFO"
    while p + 8 <= payload.len() {
        let id = [payload[p], payload[p + 1], payload[p + 2], payload[p + 3]];
        let size = u32::from_le_bytes([
            payload[p + 4],
            payload[p + 5],
            payload[p + 6],
            payload[p + 7],
        ]) as usize;
        let start = p + 8;
        let end = start
            .checked_add(size)
            .filter(|end| *end <= payload.len())
            .ok_or_else(|| {
                "its RIFF INFO list could not be read: one of its entries says it is longer than \
             the list"
                    .to_string()
            })?;
        entries.push(InfoEntry {
            id,
            data: payload[start..end].to_vec(),
        });
        p = end + (size & 1);
    }
    Ok(entries)
}

/// The entries of a whole `LIST INFO` chunk held in memory — what `lofty`
/// produces when asked to write a RIFF INFO tag into a buffer instead of a
/// file (`TagExt::dump_to`). Empty input (what `lofty` writes for a list with
/// no entries left) gives no entries.
///
/// # Errors
/// When `bytes` is not a `LIST INFO` chunk, or an entry runs past its end.
pub(crate) fn entries_from_list_chunk(bytes: &[u8]) -> Result<Vec<InfoEntry>, String> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if bytes.len() < 12 || &bytes[0..4] != b"LIST" || &bytes[8..12] != b"INFO" {
        return Err("the RIFF INFO list about to be written could not be checked".to_string());
    }
    parse_info_payload(&bytes[8..])
}

/// What one entry the save was asked to change must hold once it is done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Expectation {
    /// Exactly one entry with this id, holding this text (a zero byte or two
    /// at its end, as RIFF INFO writers add, is allowed).
    Holds(String),
    /// No entry with this id at all (the field was cleared or removed).
    Absent,
}

/// The entries one save is asked to change: each id, and what it must hold
/// afterwards. Every OTHER entry must come through byte for byte.
pub(crate) type AskedChanges = Vec<([u8; 4], Expectation)>;

/// The refusal for a save that would rewrite the RIFF INFO list of a file
/// with more than one (issue #259; the stand-in review of round 6, M1 and
/// N6) — said plainly, so a person knows what is unusual about their file.
pub(crate) fn two_lists_refusal(list_count: usize) -> String {
    format!(
        "this file has {list_count} RIFF INFO lists, and MeedyaManager's tag library would \
         merge them into one when saving, changing entries nobody asked to change, so the \
         change was refused"
    )
}

/// Compare a RIFF INFO list before and after a save.
///
/// Every entry whose id is not in `asked` must be byte-for-byte the same and
/// in the same order; an asked-for entry may move, but must satisfy its
/// [`Expectation`].
///
/// # Errors
/// A plain-English reason naming what would be lost or changed, for the
/// caller to refuse the save with.
pub(crate) fn check_entries_kept(
    before: &[InfoEntry],
    after: &[InfoEntry],
    asked: &[([u8; 4], Expectation)],
) -> Result<(), String> {
    let is_asked = |entry: &InfoEntry| asked.iter().any(|(id, _)| *id == entry.id);
    let others = |entries: &[InfoEntry]| -> Vec<InfoEntry> {
        entries.iter().filter(|e| !is_asked(e)).cloned().collect()
    };
    let others_before = others(before);
    let others_after = others(after);

    let kept_in_order = others_before.len() == others_after.len()
        && others_before
            .iter()
            .zip(&others_after)
            .all(|(was, now)| kept_as_it_was(was, now));
    if !kept_in_order {
        return Err(format!(
            "saving would {}, so the change was refused",
            what_would_change(&others_before, &others_after)
        ));
    }

    for (id, expected) in asked {
        let found: Vec<&InfoEntry> = after.iter().filter(|e| e.id == *id).collect();
        let as_expected = match expected {
            Expectation::Absent => found.is_empty(),
            Expectation::Holds(text) => {
                found.len() == 1 && trim_trailing_zeros(&found[0].data) == text.as_bytes()
            }
        };
        if !as_expected {
            let done = match expected {
                Expectation::Absent => "removed from",
                Expectation::Holds(_) => "written into",
            };
            return Err(format!(
                "{} could not be {done} the file's RIFF INFO list as asked, so the change was \
                 refused",
                describe_entry(id)
            ));
        }
    }
    Ok(())
}

/// Whether `now` is `was` come through a save: the same id, and the same
/// bytes — or the same bytes with one zero byte added at the end, when `was`
/// had none (the stand-in review of round 6, L2).
///
/// Why that one difference is allowed: RIFF INFO text ends at a zero byte,
/// and some writers leave it off the last entry or every entry. `lofty`
/// reads such an entry perfectly well and writes it back with the zero byte
/// (and, when that makes it odd in length, the padding byte after it, which
/// the entry's own size does not count and so is not part of `data`).
/// Reproduced with the `meedya` binary built from `49cec29` on a WAV whose
/// UTF-8 entries `INAM` "Song" and `ILNG` "fre" had no zero byte: setting
/// the language was refused, and the message said the tag library "cannot
/// read" the title — which was false. Every other difference still counts,
/// including a zero byte taken AWAY (`Old\0\0` becoming `Old\0`): that is
/// not this case, and telling which other differences are harmless would be
/// guessing.
fn kept_as_it_was(was: &InfoEntry, now: &InfoEntry) -> bool {
    if was.id != now.id {
        return false;
    }
    now.data == was.data
        || (was.data.last() != Some(&0)
            && now.data.len() == was.data.len() + 1
            && now.data.starts_with(&was.data)
            && now.data.last() == Some(&0))
}

/// What a save would do to the entries it was not asked to change, in
/// words, for a refusal — called only when [`kept_as_it_was`] does not hold
/// for them all, in order.
///
/// An entry missing entirely from what would be written is "lost", and only
/// then is the reason given that the tag library cannot read it as it is
/// stored: that is the one way `lofty` drops an entry it was not asked to
/// change. An entry still there with other bytes is "rewritten with
/// different bytes" — the stand-in review of round 6 (L2) found the old
/// message gave the "cannot read" reason for that too, which was untrue.
/// Same entries in another order, or one added: said as that.
fn what_would_change(before: &[InfoEntry], after: &[InfoEntry]) -> String {
    let mut remaining: Vec<&InfoEntry> = after.iter().collect();
    let mut unmatched: Vec<&InfoEntry> = Vec::new();
    for entry in before {
        if let Some(at) = remaining.iter().position(|e| kept_as_it_was(entry, e)) {
            remaining.remove(at);
        } else {
            unmatched.push(entry);
        }
    }
    let mut lost: Vec<String> = Vec::new();
    let mut rewritten: Vec<String> = Vec::new();
    for entry in unmatched {
        if let Some(at) = remaining.iter().position(|e| e.id == entry.id) {
            remaining.remove(at);
            rewritten.push(describe_entry(&entry.id));
        } else {
            lost.push(describe_entry(&entry.id));
        }
    }
    let mut parts: Vec<String> = Vec::new();
    if !lost.is_empty() {
        parts.push(format!(
            "lose {} from the file's RIFF INFO list (MeedyaManager's tag library cannot read \
             {} as {} stored)",
            join_names(&lost),
            if lost.len() == 1 { "it" } else { "them" },
            if lost.len() == 1 { "it is" } else { "they are" }
        ));
    }
    if !rewritten.is_empty() {
        parts.push(format!(
            "rewrite {} in the file's RIFF INFO list with different bytes",
            join_names(&rewritten)
        ));
    }
    if parts.is_empty() {
        // Same entries, different order, or something added.
        "change the order of, or add to, the other entries in the file's RIFF INFO list".to_string()
    } else {
        parts.join(", and ")
    }
}

/// Names joined as a person would say them: "a", "a and b", "a, b and c".
fn join_names(names: &[String]) -> String {
    match names.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
    }
}

/// `data` without the zero bytes RIFF INFO writers put at the end of text.
fn trim_trailing_zeros(data: &[u8]) -> &[u8] {
    let end = data.iter().rposition(|b| *b != 0).map_or(0, |i| i + 1);
    &data[..end]
}

/// A plain-English name for an entry: what it is, if it is one of the
/// common ones, and its id (`the title (INAM)`).
pub(crate) fn describe_entry(id: &[u8; 4]) -> String {
    let what = match id {
        b"INAM" => Some("the title"),
        b"IART" => Some("the artist"),
        b"IPRD" => Some("the album"),
        b"ICMT" => Some("the comment"),
        b"ICRD" => Some("the date"),
        b"IGNR" => Some("the genre"),
        b"ICOP" => Some("the copyright notice"),
        b"ISFT" => Some("the software name"),
        b"IENG" => Some("the engineer"),
        b"ITCH" => Some("the technician"),
        b"IKEY" => Some("the keywords"),
        b"ISBJ" => Some("the subject"),
        b"IPRT" | b"ITRK" => Some("the track number"),
        b"IMUS" => Some("the composer"),
        b"IWRI" => Some("the writer"),
        b"ILNG" => Some("the language"),
        _ => None,
    };
    let shown = InfoEntry {
        id: *id,
        data: Vec::new(),
    }
    .id_text();
    match what {
        Some(what) => format!("{what} ({shown})"),
        None => format!("the entry {shown}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &[u8; 4], data: &[u8]) -> InfoEntry {
        InfoEntry {
            id: *id,
            data: data.to_vec(),
        }
    }

    /// The language entry is asked to hold `text`.
    fn holds(text: &str) -> AskedChanges {
        vec![(LANGUAGE_ID, Expectation::Holds(text.to_string()))]
    }

    /// The language entry is asked to be gone.
    fn language_absent() -> AskedChanges {
        vec![(LANGUAGE_ID, Expectation::Absent)]
    }

    #[test]
    fn identical_other_entries_and_the_asked_language_pass() {
        let before = [entry(b"ILNG", b"fre\0"), entry(b"INAM", b"Old\0")];
        let after = [entry(b"INAM", b"Old\0"), entry(b"ILNG", b"en\0")];
        assert_eq!(check_entries_kept(&before, &after, &holds("en")), Ok(()));
    }

    #[test]
    fn a_lost_entry_is_refused_and_named() {
        let before = [entry(b"ILNG", b"fre\0"), entry(b"INAM", b"Caf\xe9\0")];
        let after = [entry(b"ILNG", b"en\0")];
        let message = check_entries_kept(&before, &after, &holds("en")).unwrap_err();
        assert!(message.contains("the title (INAM)"), "{message}");
        assert!(message.contains("the change was refused"), "{message}");
    }

    /// An entry still there with other bytes is said to be rewritten, not
    /// lost — and not blamed on the tag library being unable to read it
    /// (the stand-in review of round 6, L2: that reason was untrue here). A
    /// zero byte taken away is still a difference.
    #[test]
    fn a_changed_entry_is_refused() {
        let before = [entry(b"INAM", b"Old\0\0"), entry(b"ILNG", b"fre\0")];
        let after = [entry(b"INAM", b"Old\0"), entry(b"ILNG", b"en\0")];
        let message = check_entries_kept(&before, &after, &holds("en")).unwrap_err();
        assert!(
            message.contains(
                "rewrite the title (INAM) in the file's RIFF INFO list with different bytes"
            ),
            "{message}"
        );
        assert!(!message.contains("cannot read"), "{message}");
    }

    /// The stand-in review of round 6, L2: an entry with no zero byte at
    /// its end, rewritten with one, is not a loss — the save goes ahead.
    /// Any other added byte is still refused.
    #[test]
    fn a_final_zero_byte_added_is_not_a_loss() {
        let before = [entry(b"INAM", b"Song"), entry(b"ILNG", b"fre")];
        let after = [entry(b"INAM", b"Song\0"), entry(b"ILNG", b"en\0")];
        assert_eq!(check_entries_kept(&before, &after, &holds("en")), Ok(()));
        // An empty entry gaining its zero byte is the same case.
        assert_eq!(
            check_entries_kept(&[entry(b"ICMT", b"")], &[entry(b"ICMT", b"\0")], &[]),
            Ok(())
        );
        for after in [
            entry(b"INAM", b"Song\0\0"),
            entry(b"INAM", b"Song!"),
            entry(b"INAM", b"Son\0"),
        ] {
            assert!(
                check_entries_kept(
                    &[entry(b"INAM", b"Song")],
                    std::slice::from_ref(&after),
                    &[]
                )
                .is_err(),
                "{after:?} is not the same entry with its final zero byte added"
            );
        }
    }

    /// Lost and rewritten entries in one save are each named as what
    /// happens to them.
    #[test]
    fn lost_and_rewritten_entries_are_named_apart() {
        let before = [entry(b"INAM", b"Caf\xe9\0"), entry(b"IART", b"A\0\0")];
        let after = [entry(b"IART", b"A\0")];
        let message = check_entries_kept(&before, &after, &[]).unwrap_err();
        assert!(
            message.contains(
                "lose the title (INAM) from the file's RIFF INFO list (MeedyaManager's tag \
                 library cannot read it as it is stored), and rewrite the artist (IART)"
            ),
            "{message}"
        );
    }

    #[test]
    fn a_reordered_entry_is_refused() {
        let before = [entry(b"INAM", b"A\0"), entry(b"IART", b"B\0")];
        let after = [entry(b"IART", b"B\0"), entry(b"INAM", b"A\0")];
        let message = check_entries_kept(&before, &after, &language_absent()).unwrap_err();
        assert!(message.contains("order"), "{message}");
    }

    #[test]
    fn an_added_entry_is_refused() {
        let before = [entry(b"INAM", b"A\0")];
        let after = [entry(b"INAM", b"A\0"), entry(b"IART", b"B\0")];
        assert!(check_entries_kept(&before, &after, &language_absent()).is_err());
    }

    #[test]
    fn the_language_must_hold_exactly_what_was_asked() {
        let before = [entry(b"ILNG", b"fre\0")];
        // Wrong text, two language entries, or none at all: each refused.
        for after in [
            vec![entry(b"ILNG", b"fr\0")],
            vec![entry(b"ILNG", b"en\0"), entry(b"ILNG", b"en\0")],
            vec![],
        ] {
            assert!(
                check_entries_kept(&before, &after, &holds("en")).is_err(),
                "{after:?} must not count as holding \"en\""
            );
        }
        // A clear must leave none.
        assert!(
            check_entries_kept(&before, &[entry(b"ILNG", b"en\0")], &language_absent()).is_err()
        );
        assert_eq!(check_entries_kept(&before, &[], &language_absent()), Ok(()));
    }

    /// Every save of a WAV, not only a language save (the stand-in review
    /// of round 6, carry-over 1): a removed field must be gone, and nothing
    /// else may be lost — named as what it is.
    #[test]
    fn a_removed_field_must_be_gone_and_nothing_else_lost() {
        let asked = vec![(*b"IART", Expectation::Absent)];
        let before = [entry(b"IART", b"A\0"), entry(b"INAM", b"T\0")];
        assert_eq!(
            check_entries_kept(&before, &[entry(b"INAM", b"T\0")], &asked),
            Ok(())
        );
        let still_there = check_entries_kept(&before, &before, &asked).unwrap_err();
        assert!(
            still_there.contains("the artist (IART) could not be removed"),
            "{still_there}"
        );
        let title_lost = check_entries_kept(&before, &[], &asked).unwrap_err();
        assert!(title_lost.contains("the title (INAM)"), "{title_lost}");
    }

    #[test]
    fn a_list_chunk_in_memory_is_read_with_its_padding() {
        // "Café" (5 bytes with its zero) is odd, so one padding byte follows.
        let mut chunk = b"LIST".to_vec();
        let mut body = b"INFO".to_vec();
        body.extend(b"INAM");
        body.extend(5u32.to_le_bytes());
        body.extend(b"Caf\xe9\0\0");
        body.extend(b"ILNG");
        body.extend(4u32.to_le_bytes());
        body.extend(b"fre\0");
        chunk.extend(u32::try_from(body.len()).unwrap().to_le_bytes());
        chunk.extend(body);
        assert_eq!(
            entries_from_list_chunk(&chunk).unwrap(),
            vec![entry(b"INAM", b"Caf\xe9\0"), entry(b"ILNG", b"fre\0")]
        );
        assert_eq!(entries_from_list_chunk(&[]).unwrap(), Vec::new());
        assert!(entries_from_list_chunk(b"RIFF").is_err());
    }

    #[test]
    fn an_entry_longer_than_its_list_is_an_error() {
        let mut body = b"INFO".to_vec();
        body.extend(b"INAM");
        body.extend(100u32.to_le_bytes());
        body.extend(b"short");
        assert!(parse_info_payload(&body).is_err());
    }
}
