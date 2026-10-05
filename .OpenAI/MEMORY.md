# MeedyaManager — Codex memory

> **(C) 2025-2026 MWBM Partners Ltd**
>
> Facts and lessons that a fresh session is most likely to get wrong. Checked on 2026-09-23;
> the last five lessons were added on 2026-10-04.
> For the live state of the work, `.claude/HANDOFF.md` §0 always wins over this file.

## Where the work happens

- **Working branch:** `claude/musicbrainz-api-migration-7jxszn`. It will be merged into `alpha`
  through **one** pull request, opened later when the owner says. Do not open others.
- Commit each finished task to that branch. **Do not push unless the owner asks**; a request
  to push covers that occasion only (owner, 2026-09-23).
- Cargo is not on the default `PATH`: `export PATH="$HOME/.cargo/bin:$PATH"`.
- **The language-policy work (#251) is on a separate branch, `feature/bcp47-language-policy`**,
  cut from the working branch above on purpose. Its own note sits at the top of
  `.claude/HANDOFF.md`, ahead of §0; it does not change anything in §0.

## What is really finished (do not overstate it)

- `meedya lookup`, `export` and `serve` are stubs that exit with code 3 ("not implemented").
- Cloud monitoring (M7), database export (M9) and the media server (M10) are scaffolding only.
  There is **no working API and no website**, so there is nothing yet for OpenAPI or Swagger
  to describe.
- The macOS app does not link the Rust engine yet (#66).

## Open dangers on the branch (2026-09-23)

- **The organiser (`meedya watch --organize`, #180) had known data-loss bugs and was committed
  as `6ab0c8d` without the fixes a review asked for.** As of 2026-09-23 the owner has switched
  it off with a safety catch: `watch --organize` without the global `--dry-run` flag now refuses
  (prints an error, exits code 3, moves nothing) instead of running for real, and
  `service install` on Linux/macOS refuses the same way instead of registering a service that
  would call it. `--dry-run` previews still work. This is controlled by one constant,
  `ORGANISING_SWITCHED_ON = false`, in `crates/mm-cli/src/commands/watch.rs`. See handoff §0.
- **P0 #233:** renaming loses the file extension when the new name contains a full stop, with
  default settings. The fix was built once but lost; it must be rebuilt.
- **P0 #222:** fixed on macOS and Windows, but the Windows fix has never been compiled.

## Lessons learned

- **Verify against the code, never against the documents.** Three milestones were once closed
  as complete when they were only scaffolding.
- **A module with zero callers is not finished**, however well tested.
- **Classify files by what they mean to a media library.** A disc image (`.iso`, `.cue`/`.bin`,
  and so on) is a record, not an installer. A `.cue` and its `.bin` must stay together.
- **Anything only on one machine can vanish** — temporary work folders, scratch files, unpushed
  commits. Commit finished work, record unfinished work in the handoff, and tell the owner what
  is waiting to be pushed.
- `grep -c` inside a chain of `&&` commands stops the chain when the count is zero; add `|| true`.
- **Test the whole chain, not only each link.** In the language-policy work, one test proved
  the plan held a note and another proved the printer printed a note it was handed; the step
  between them could be broken with every test still passing. Break the code on purpose
  (a "mutation") to find out whether a test really guards it.
- **A message about what was stored must be checked against the file.** A note that sounded
  right ("replaced with the current code") was false for everyday input; reading the file back
  with an independent tool (mutagen) is what showed it.
- **A preview must read the file the save will change.** With Test Mode on and a copy already
  made, a save changes the copy, not the file named on the command line; a note worked out from
  the original was wrong about the copy. `integrity::where_a_save_starts` answers "which file";
  use it for anything that previews a save.
- **The per-commit checks do not build the documentation.** In the language work a public doc
  comment linking to a private function passed fmt, clippy and every test, and was caught only
  by `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` in the full check list. Run
  that before calling a round finished.
- **A handoff should not say what is on GitHub.** Push-status lines in the language note went
  stale more than once; git (`git status -sb`) is the record. The note keeps only a table of
  finished reviews and which commits each covered.
- **A library that rewrites a whole structure can drop what it could not read.** In the
  language work, the tag library could not read a WAV's RIFF INFO title written in Latin-1,
  left it out of what it read, and wrote the list back without it — a language save deleted
  the title and reported success. Language saves now read that list's raw bytes before and
  after and refuse any other change (`metadata/riff_info.rs`). Since the stand-in review of
  round 6 every save of a WAV is guarded, and a removal rewrites only the tag sections that hold
  the field — rewriting a section that holds nothing to remove can only lose something.
- **A reader's rule is not a writer's rule.** The shared language reader takes the first of
  several values separated by a zero character — right for reading a stored field, wrong for a
  value being set, where it silently threw the rest away. Check which side a function was
  written for before reusing it on the other.
- **"Last one wins" hides a lost write.** `meedya edit --set x=1 --set x=2` reported both as
  done. Refusing the ambiguity is the answer that cannot mislead.
- **Trim exactly what the standard says.** Rust's `str::trim` removes a no-break space and other
  Unicode spaces; the language policy keeps them (they make a value malformed). Use the policy's
  own four characters (`trim_lang_whitespace`).
- **Codex reviews here when it can.** Its catch-up review of the whole language branch (36
  commits) found six problems four earlier rounds had not; it traced code but could not run it,
  so each finding was reproduced on a real file before it was fixed.
- **A check after a save is proof, not protection.** A refusal from the check after saving comes
  after the save; in Test Mode, once an earlier edit made the copy, that save went into the
  copy. Everything a check can predict must be checked BEFORE the first write (the two-INFO-list
  WAV, #259, slipped through exactly this way).
- **A dry run must run the same checks as the real run.** A check that lives only inside the
  save is invisible to `--dry-run`; expose a read-only form and call it when the plan is built.
- **A refusal message is only safe if it shows what was refused.** Quoting a value raw hides a
  zero-width space, a no-break space or a direction-changing character — exactly the things
  that made it wrong. Write out everything not plainly visible.
- **Prove a test gap with the reviewer's own planted fault.** Two of eleven planted faults
  turned no test red; the new tests were each shown red against that exact fault.
