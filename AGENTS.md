# MeedyaManager — instructions for AI coding agents

> **(C) 2025-2026 MWBM Partners Ltd**
>
> Read by Codex and other agents that look for `AGENTS.md`. Claude Code reads
> `.claude/CLAUDE.md`, which carries the same rules in more detail. **Keep the two in step —
> if you change a rule in one, change it in the other.**

## The first thing to understand: this application is about media

MeedyaManager manages **media files and everything that belongs with them**. When you decide
how to categorise, group or handle any file type, ask:

> *"What does this mean to somebody's media library?"*

**Never** ask only *"what is this file technically?"* That question gives the wrong answer, and
it has already caused a real defect.

### What is in scope

- **Audio and video** — the primary content.
- **Raw, bit-for-bit disc images** of original media discs — Audio CD, enhanced CD, mixed-mode
  CD, DTS CD, HDCD, DVD, HD DVD, Bluray, 3D Bluray:
  `.iso` `.nrg` `.mdx` `.mds` (with `.mdf`) `.cue` `.bin` `.cdr`.
  These are **recordings**, not containers.
- **Associated documents** — booklets and liner notes as PDFs, artwork as images, and animated
  or motion artwork as MP4.
- **Companion files** — subtitles, lyrics, cue sheets, rip logs, checksum files, playlists.

### The mistake this rule exists to prevent

On 2026-09-05 the owner asked why an `.iso` was being treated as software. The answer: it was
added in commit `e80be51`, a bulk tidy-up that worked through a list of container formats and
dropped `ISO` into the Archive group beside `ZIP`, `GZ`, `MSI`, `DEB` and `APK`. Nobody decided
it — it was inherited from a checklist.

Technically it was even defensible. ISO 9660 really is a filesystem format, a way of packing
many files into one, exactly like ZIP. But it answered the wrong question. **A bit-perfect copy
of an Audio CD is a record, not an installer.**

The same mistake then produced false documentation: `README.md` and `help/faq.md` both promised
that disc images move alongside their media, because whoever wrote them described what the
product was *meant* to do while the code followed different logic entirely.

### How to apply it

- Formats that genuinely are software — `DMG`, `MSI`, `DEB`, `RPM`, `PKG`, `APK`, `JAR` — stay
  archives. They are not media.
- Anything that can hold a recording, a performance, a booklet or artwork belongs in the media
  model, with the tags and rules that implies.
- When a new format appears, decide from the library's point of view **first**, then work out
  the technical handling.
- **Be suspicious of bulk "add all the formats" passes.** That is exactly how this slipped in.

### Disc image handling rules (owner decisions, 2026-09-05)

1. **The whole containing folder moves as one sealed unit.** A rip normally carries a log, a
   checksum file and artwork that only make sense beside the image, so nothing inside is
   renamed, reordered or split off.
2. **Naming order:** read the `.cue` sheet (plain text, carries `PERFORMER` and `TITLE`) → a
   disc fingerprint looked up online, **offered as a suggestion, never applied automatically**
   → the folder name. **If none is confident, do not rename at all.**
3. **Look inside the image** to tell an audio disc from a film disc, so each follows its own
   rules.

**A `.cue` and its `.bin` must keep the same base name and stay side by side.** Separating or
renaming either one destroys the disc image. See issues #217 and #218.

## Write in plain, everyday English

Everything written for the owner — replies, progress reports, commit messages, issue comments,
documentation and in-app help — is in plain English with no jargon. Where a technical term
genuinely cannot be avoided, use it and then say in ordinary words what it means and why it
matters.

This does not lower the standard of the work. The code and the analysis stay exactly as
rigorous; only the explanation changes. Prefer more words if they make the meaning clearer.
Explain *why*, not only *what*.

When reporting on work done, be blunt about what is finished, what is not, what was not checked
and what went wrong. "I could not test this because there is no database on this machine" is
worth far more than implying something was verified.

## Verify against the code, never against the documents

This project has a documented history of its own documentation and commit messages overstating
what is finished. Three milestones were closed as complete and had to be reopened because they
were scaffolding with no working implementation.

**Assume nothing. Cite a file and line you have actually read.**

Watch especially for this pattern: **a module that is fully written, well tested, and has zero
callers.** Real examples from this repository — the logging system had no callers at all while
the command-line tool used a different one; the code that strips usernames out of logs was
tested but never once invoked; the module that recognises disc images is never called by the
code that moves files. A feature nobody can reach is not finished.

## Build and test

Cargo is not on the default `PATH`. Run `export PATH="$HOME/.cargo/bin:$PATH"` first.

    cargo test --workspace                              # 1,354 passing, 0 failing
    cargo clippy --workspace --all-targets -- -D warnings
    cargo deny check
    cd macos && swift build                             # baseline is 1 error line

For the macOS build, ignore `#Preview` / `PreviewsMacros` / "external macro" errors — those
need full Xcode rather than the Command Line Tools and only happen on a developer machine.

`crates/mm-gtk` is excluded from the workspace and **cannot be compiled on this machine** (no
GTK4). Treat it as read-only and do not change its public interface.

## House rules

- Every source file carries `// (C) 2025-2026 MWBM Partners Ltd`.
- Dense comments explaining **why**, not what.
- British English.
- Never weaken a test or silence a warning to make something pass.
- **No new dependencies without asking.** A lock-file change forces a cold rebuild on three
  operating systems, and this branch has never been through CI, so any failure would be
  impossible to attribute.
