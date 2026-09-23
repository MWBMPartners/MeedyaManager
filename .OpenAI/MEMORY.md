# MeedyaManager — Codex memory

> **(C) 2025-2026 MWBM Partners Ltd**
>
> Facts and lessons that a fresh session is most likely to get wrong. Checked on 2026-09-23.
> For the live state of the work, `.claude/HANDOFF.md` §0 always wins over this file.

## Where the work happens

- **Working branch:** `claude/musicbrainz-api-migration-7jxszn`. It will be merged into `alpha`
  through **one** pull request, opened later when the owner says. Do not open others.
- Commit **and push** each finished task to that branch (owner, 2026-09-23).
- Cargo is not on the default `PATH`: `export PATH="$HOME/.cargo/bin:$PATH"`.

## What is really finished (do not overstate it)

- `meedya lookup`, `export` and `serve` are stubs that exit with code 3 ("not implemented").
- Cloud monitoring (M7), database export (M9) and the media server (M10) are scaffolding only.
  There is **no working API and no website**, so there is nothing yet for OpenAPI or Swagger
  to describe.
- The macOS app does not link the Rust engine yet (#66).

## Open dangers on the branch (2026-09-23)

- **The organiser (`meedya watch --organize`, #180) is switched on with known data-loss bugs.**
  It was committed as `6ab0c8d` without the fixes a review asked for. See handoff §0.
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
  commits. Push finished work and record unfinished work in the handoff.
- `grep -c` inside a chain of `&&` commands stops the chain when the count is zero; add `|| true`.
