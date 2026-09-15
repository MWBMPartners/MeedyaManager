# MeedyaManager — Session Handoff

> **(C) 2025-2026 MWBM Partners Ltd**
>
> **Purpose:** the single place to look to resume work after any interruption.
> Records *verified* state only — never aspirational state. Update after every task.

**Last updated:** 2026-09-15
**Updated by:** Claude Opus 5, session `07a21012`
**Working branch:** `claude/musicbrainz-api-migration-7jxszn` → will PR into **`alpha`**
**Branch HEAD:** latest code commit `599d921` (pushed) — **plus the uncommitted organiser (#180) and its documentation; see §0 and §15**

---

## 0. Read this first — where things stand on 2026-09-15

### In one paragraph

The branch is pushed and clean up to `83628d9`. Of the four pieces of work built on top of it, the **release guard** (`b694d70`) and the **Windows
guard** (`eb2c2a9`) are now committed after review. The **write lock** (#49) and the **organiser** (#180)
remain uncommitted, because the review found them unsafe; their redesign is being planned. **Codex, the usual reviewer, cannot run until 2026-09-20 16:30**, so a fresh
Opus agent that built none of it is reviewing in its place — Fable, the first stand-in, was out of
usage credits as well. Two P0 data-loss bugs were found and
fixed this fortnight: #219 is committed, and #222 is committed for macOS with its Windows half in
the uncommitted work. On 2026-09-15 the owner decided **the Apple app stays macOS-only** (#228, iPhone Duo, closed
as not planned), and that a **side-loadable Android phone app** is wanted (#230), with foldable
support (#229) on top. **No Android code exists yet.**

### The uncommitted work — do not commit it without a review

| Piece | Issue | Built by | State |
| --- | --- | --- | --- |
| The Windows app cannot move files without its engine | #222 | Sonnet agent | ✅ **Committed `eb2c2a9`** after review, with the Test Mode promise fixed. **Never compiled** — no .NET here; #222 stays open until verified on Windows. |
| The release pipeline stops attaching an engine-less macOS app | #222 | Sonnet agent | ✅ **Committed `b694d70`**, with the engine-check fix. `actionlint` clean. Has never run. |
| The write lock is taken before renames; the unused `AppState` is deleted | #49 | Opus, then redesigned by Sonnet | ✅ **Committed `599d921`** as an operating-system file lock, after review and fix-ups. #49 stays open until the help pages are rewritten (stage f). |
| `watch --organize` made real; the background service made honest | #180 | Opus agent | **The agent ran out of credit part-way and never reported.** The code is complete and tested; **its documentation was never written.** |

Gate on the working tree as it stands, 2026-09-15:

```text
cargo fmt --all --check                      clean (after cargo fmt --all)
cargo clippy --workspace --all-targets       0 issues with -D warnings
cargo test --workspace                       1,402 passed, 0 failed — run twice, identical
RUSTDOCFLAGS="-D warnings" cargo doc         passes
cargo deny check                             FAILS: RUSTSEC-2026-0285 in rustls (#227) — not caused by this work
Cargo.toml / Cargo.lock                      untouched
```

The organiser was also checked by hand, isolated from real settings with an empty
`MM_CONFIG_DIR`: with no watch folders it now gives a plain error and exits 1 (it used to exit 3),
and a dry-run sweep of a folder moved nothing.

**#180 documentation — now written, not yet committed.** A Sonnet documentation agent updated
`help/background-service.md`, `help/getting-started.md`, `help/faq.md`, `docs/api/cli.md`,
`help/cli-reference.md`, `docs/changelog.md`, `Dev_Notes.md`, `PROJECT_STATUS.md` and `README.md`,
citing the source line behind each claim. It was instructed not to touch code.

**Two things found while checking that documentation — settle both before committing #180:**

- **The organiser ignores the configured output folder.** In `crates/mm-cli/src/commands/watch.rs`,
  `organise_directory` passes `output_dir: Some(root)` — always the watched folder — so the
  `rename.output_dir` setting and `--output-dir` have no effect on it. The plan was to honour the
  setting and fall back to the watched folder only when none is set. The new documentation
  accurately describes the code *as it is*, so if the code changes, that page must change with it.
- **Files from a folder whose run only partly succeeds are not retried.** `organise_settled` retries
  only when the scan returns `ERROR`, which usually means the write lock is held. A conflict or a
  single failed move returns `PARTIAL`, and those files leave the queue with no extra message from
  the organiser. Not retrying a conflict is reasonable, because waiting will not resolve it — but
  confirm the scan's own output makes the outcome visible in the service log.

### Review status — read before trusting any of the above

- **Codex has reviewed none of this.** The first attempt hung, because `codex exec` waits for
  extra input unless it is given `< /dev/null`. The second hit Codex's usage limit: *"try again
  at Sep 20th, 2026 4:30 PM"*.
- **First stand-in, Fable, failed at once:** out of usage credits on 2026-09-15.
- **Fable failed again** later that day when asked to plan the fixes, so **Opus is planning in its place.** Try Fable first on the next planning run.
- **Second stand-in, Opus — review complete (see the verdict below):** a fresh Opus agent (the dev-team `opus-builder`, in the reviewer
  role), working from the snapshot `review-snapshot.patch` (§5). Its verdict decides what gets
  committed. **Its independence is uneven, and that must stay visible:**
  - For the **Windows guard** and **release guard**, built by Sonnet, Opus is a different model —
    genuinely independent.
  - For the **write lock** and the **organiser**, built by Opus, it is the **same model**. Its only
    independence is having no memory of building them. Two instances of one model tend to make
    the same mistake in the same place, so these two are **the first priority for Codex's
    catch-up review**.
- **Owed:** a full Codex catch-up review once Codex is available. No Codex review has completed
  at all in this session, so the safe scope is every commit from `c68719c` onward, plus the four
  pieces above — and ideally the whole branch.
- Claude also hit its **monthly spend limit** shortly after 2026-09-07; that is what stopped the
  organiser agent. Three limit messages in one fortnight — two from Claude, one from Codex — suggests a limit
  needs raising, rather than the work needing to be re-planned.

### Stand-in review verdict — 2026-09-15

Opus, standing in for Codex and Fable, reviewed the four pieces against the snapshot. It changed
nothing, and ran only previews and scratch-folder tests. **Verified by the orchestrator on 2026-09-15**, by reproducing the problem or reading the code:
#1 — a preview moved `Album.cue` and `Album.log` away while `Album.bin` was still downloading,
**and planned to move the half-downloaded `Album.bin.crdownload` itself**, a further defect in
the already-committed `scan` (now #231); #2 — a preview planned `01 - song.wav → 01 - 01 - song.wav`;
#3 — the prompt mentions only files "as they arrive"; #4; #5; #6; #8 — `nm | grep -q` said
"not linked" five times out of five with the symbol present, while reading from a file said
"linked" five out of five; and #10 (now #232). **Not independently verified:** the Ctrl+C part of
#3, and #7, #11–#17. Its full report is in this
session's subagent transcripts.

| Piece | Safe to commit? | Blocking findings |
| --- | --- | --- |
| Release guard (`release.yml`) | **Yes** | none — but fix #8 in the same commit |
| Windows guard | **No** | #5 |
| Write lock (#49) | **No** | #6, #7, #16 — replace with an operating-system file lock (`File::try_lock`, available in the pinned Rust 1.98) |
| Organiser and service (#180) | **No** | #1, #2, #3, #4 |

| # | Severity | Piece | Finding |
| --- | --- | --- | --- |
| 1 | **Critical** | Organiser | A `.cue` whose `.bin` has not arrived under its final name yet — still downloading, or mid-copy under a temporary name — counts as a loose file and is moved, destroying the disc image. Also affects plain `scan` on an incomplete rip. A comment in `disc/mod.rs` claiming a cue sheet is never renamed on its own is false. |
| 2 | **Critical** | Organiser | Templates built on `<Filename>` do not settle: the organiser's own move raises a new event and renames again (`01 - 01 - 01 - song.wav`). Forcing "skip" does not stop it. |
| 3 | High | Organiser | Starting it reorganises the whole existing library at once; the prompt mentions only files "as they arrive"; Ctrl+C does not stop the sweep. |
| 4 | High | Service | macOS `service install` starts the service immediately (`RunAtLoad` plus `launchctl load`) while telling the user it has not started. |
| 5 | High | Windows | Test Mode is only a flag inside the app, never passed to the engine, so tag saves rewrite the real file — and this patch added a dialog promising edits go to copies. |
| 6 | High | Lock | Two processes can both hold the lock: clearing a leftover lock is several separate steps, and `release` deletes whatever lock file is present, even another process's. Demonstrated with the real code. |
| 7 | Medium | Lock | A lock file naming a process number since reused by an unrelated program blocks renaming forever. Release builds abort on panic, so a crash leaves the file behind. |
| 8 | Medium | Release | The "does the app link the engine?" check reports "no" even when it does (`nm \| grep -q` under `pipefail`). Fails safe, but the app would never be attached once #66 lands. |
| 9 | Medium | Windows, docs | Windows Execute does not take the write lock, but the new documentation says the desktop apps do. |
| 10 | Medium | Windows (older code) | Settings "Save" replaces the whole `settings.json5` with five keys, erasing templates, watch folders and rules. Needs its own issue. |
| 11 | Medium | Organiser | "Settled" means only "no events for 2 seconds"; a slow copy can be organised half-written. |
| 12 | Low–medium | Organiser | Lock-busy retries never give up and fill the log; the start-up sweep never retries. |
| 13 | Low–medium | Organiser | Ignores the configured output folder; nested watch folders disagree about who owns a file. |
| 14 | Low–medium | Service | The macOS agent logs to fixed file names in `/tmp`, which another account on a shared Mac can redirect. |
| 15 | Low | Organiser | Watch folders given through a symlink or relative path are swept once, then ignored on macOS. |
| 16 | Low | Lock | A failed write leaves an empty lock file that blocks every later run. |
| 17 | Low | Service | Unquoted path in the systemd unit, unescaped XML in the macOS agent file, restart loop when a watched drive is unplugged. |
| 18 | Low | Tests, wording | The C# test checks a copy of the logic; several stale comments; release-notes wording; tests changing a shared environment variable. |

**What the reviewer confirmed is correct:** the lock is held for the whole function and taken before
the scan; "skip" really is forced on every organiser path; `--dry-run` moves nothing; every installed
service command includes `--yes`; Windows refuses service install honestly; the C# reads as though
it would compile.

### Open P0s

- **#233** — renaming drops the file extension whenever the new name contains a full stop, **with
  default settings**. Every default template — the engine's, `meedya config init`'s and all three apps' —
  omits the extension token; only the repository's *example* `config/settings.json5` is safe. Being fixed
  in stage (a) (§15).
- **#222** — the apps renamed real files to "[Preview] …". macOS fixed in `83628d9`, Windows in
  `eb2c2a9`, and the release guard in `b694d70`. Stays open until the Windows fix is tried on a real
  Windows machine.
- **#19** — labelled P0: the Python archive tag does not exist. Owner decision: create it, or
  close it as not planned.

### What to do next

See §15 for the fix plan now being carried out, and §8. **Where it stands right now (2026-09-15, midday):**

- Stages (a) and (b) are the only ones started. **(b), the lock, is committed** (`599d921`).
- **(a) is being rebuilt** with the review's fixes, by a Sonnet agent in the worktree
  `.claude/worktrees/agent-ac1c1ad32f50eb48f`. After the rebuild comes an Opus re-review, then its
  two commits are copied onto the branch: one for #231 and #233, one for rustls (#227). Then the
  worktree is deleted, which frees about 9 GB.
- **The issue sweep is being refreshed.** The 2026-09-06 report was checked against `04f8ddf`, and 21
  commits have landed since. An Opus agent (Fable was out of credits for the fourth time) is
  re-checking every verdict against HEAD `7be7e1c`. It writes `sweep-actions.json`,
  `sweep-new-issues.json` and `sweep-refresh-summary.md` to the session scratchpad, and changes
  nothing on GitHub.
  - The orchestrator reviews that output, then runs `post-sweep.py` from the same folder: a dry run
    first, then `--execute`.
  - The script checks every entry before it writes anything, and is safe to re-run after an
    interruption.
  - **If the scratchpad has gone, re-run the refresh from §14a's verdicts**; the brief is in this
    session's transcript.
- Then stages (c)–(g), and the Codex catch-up review once Codex is available (after 2026-09-20
  16:30).

---

## 1. Status board

Commissioned 2026-09-02 as a full project-state reconciliation, then extended into alpha-readiness
work. The detail for each row is in the section it names.

| # | Step | State |
| --- | ------ | ------- |
| 1–12 | Reconciliation: reconnaissance, owner decisions, deep audit, first issue sweep, `main` merge, MusicBrainz port, documentation rewrite, branch consolidation | ✅ done (§4, §6, §9) |
| 13 | **Round 1** — seven alpha-readiness packages | ✅ done (§10, §10b) |
| 13b | **Round 2** — release readiness; version cut to `1.4.0-alpha.1` | ✅ done (§10c) |
| 13c | **Round 3** — engine bridge package, honest stubs, API docs | ✅ done — but it broke the macOS build (1 → 841 errors), fixed in `9e61ee0` (§12a) |
| 13d | **Round 4** — TheTVDB login (#210), watcher debounce (#45), file logging (#50) | ✅ done (§12b) |
| 13e | Disc images #217 stages 1–3, and the P0 data loss #219 | ✅ done — `c68719c`, `36d737e`, `04f8ddf` |
| 13f | Second full issue sweep — all 207 issues, against code **and** upstream | ✅ analysis done 2026-09-06 (§14a) · ⏳ **the per-issue comments have not been posted** |
| 13g | P0 #222 — the macOS app renamed real files to "[Preview] …" | ✅ fixed `83628d9` |
| 13h | #222 Windows half and release guard | ✅ committed `eb2c2a9` and `b694d70` after review · the Windows code has never been compiled |
| 13h2 | #49 write lock and #180 organiser | 🔄 **review verdict: not safe** · fixes being planned (Opus, standing in for Fable) |
| 13i | Security advisory #227 (`rustls`) | ⏳ after 13h, in a commit of its own |
| 13j | Mobile platforms | ✅ decided 2026-09-15: **the Apple app stays macOS-only** (#228 closed) · 📝 **Android phone app** #230, side-loadable, with foldables in #229 · no Android code yet |
| 14 | **Open the PR to `alpha`** | ⏳ owner's call — not created, per the no-PR-stacking rule |
| 15 | Post-PR dev-cache cleanup (per `.claude/CLAUDE.md`) | ⏳ after the PR exists |

**No CI has ever run on this branch.** The first pull request will be its first run, so expect
surprises on Linux in particular: the GTK crate has been edited repeatedly and cannot be compiled
on this machine.

### Commits since the end of Round 2 (`9f3719b`) — all pushed

| Commit | Date | Summary |
| --- | --- | --- |
| `6dec347` | 2026-09-03 | docs(claude): record Round 2 outcomes in the handoff |
| `5655da5` | 2026-09-03 | style(gtk): format mm-gtk and add a formatting gate to Linux CI |
| `4ab35aa` | 2026-09-03 | docs(claude): record the branch-alignment analysis and plan |
| `a63432b` | 2026-09-03 | docs: bring documentation in line with Rounds 1 and 2 |
| `633f8e3` | 2026-09-03 | fix: Round 3 — mm-gtk compiles, macOS rename identity, Test Mode protection, real FFI header |
| `134c325` | 2026-09-03 | docs(api): add API documentation for the three real API surfaces |
| `9e61ee0` | 2026-09-03 | fix(macos): exclude the generated UniFFI bindings from the SwiftPM target |
| `2e1cb6c` | 2026-09-03 | build(gtk): commit the mm-gtk lockfile, as .gitignore already intends |
| `2027096` | 2026-09-03 | fix(providers): exchange the TheTVDB API key for a JWT before searching (#210) |
| `7bd4a0f` | 2026-09-03 | fix(core): actually debounce filesystem events (#45, debounce half) |
| `db33e85` | 2026-09-03 | feat(logging): wire up file logging and apply the PII redaction (#50) |
| `f33e80b` | 2026-09-05 | docs(claude): record the plain-English rule and the disc-image requirement |
| `3a9e10d` | 2026-09-05 | docs(handoff): record the Round 3 regression, Round 4, and the disc-image gap |
| `83d8776` | 2026-09-06 | docs: record why an ISO was mistaken for software, so it cannot happen again |
| `d3512e9` | 2026-09-06 | docs: always check MeedyaSuite-core before saying a feature is missing |
| `8729e48` | 2026-09-06 | docs(handoff): write down the issue-sweep specification before running it |
| `c68719c` | 2026-09-06 | feat(core): disc images are media, not archives (#217, stage 1 of 3) |
| `36d737e` | 2026-09-06 | feat(core): read cue sheets, so a disc can name itself (#217, stage 2 of 3) |
| `04f8ddf` | 2026-09-06 | fix(cli): stop scan --execute destroying disc images (#219, P0) |
| `83628d9` | 2026-09-07 | fix(macos): an unlinked build can no longer rename a file (#222, P0) |

### Reconciliation and Round 1 commits

| Commit | Summary |
| --- | --- |
| `939c6af` | `.claude/HANDOFF.md` created |
| `58ea003` | Merge `main`; port `musicbrainz.rs` onto the upstream provider API |
| `2b08392` | Rewire MusicBrainz/ISRC/ISWC through the hardened seam (#198) |
| `a4d45dc` | Documentation rewrite — 32 files, 121 audited inaccuracies |
| `b0bb469` | Test-count corrections + changelog historical caveat |
| `6f69478` | All 19 `help/providers/` pages rewritten |
| `1767a76` | Handoff finalised for the reconciliation session |
| `437f286` | Merge `claude/issue-196-identifier-convergence` (closes #196) |
| `b42e8b4` | Consolidate `.claude/` config from the two stale config branches |
| `1f8d31a` | Carry over the last 5 relevant `settings.local.json` permissions |
| `03823f0` | Record the Round 1 work-package plan |
| `632b5a3` | **#207** LICENSE + package staging |
| `85c9285` | **#212** one config-directory resolver + pre-existing flaky-test fix |
| `44b9ad8` | **#205** CLI stubs return exit 3, not fake success |
| `f74869b` | **#211** shipped settings match `AppConfig`; CI drift guard |
| `b5bb6c9` | **#205** nine UI views marked preview-only |
| `6894105` | **#128** Test Mode enforced on every write path |
| `cf7a6ec` | **#201** scan data loss + path traversal closed |

### Latest verification

At the last commit, `83628d9`. That commit changed only Swift, so the Rust figures are those
measured at `04f8ddf`:

```text
cargo test --workspace                               1,395 passed, 0 failed
cargo clippy --workspace --all-targets -D warnings   0 issues
RUSTDOCFLAGS="-D warnings" cargo doc                 passes
swift build (macOS)                  1 error line = correct baseline (#Preview needs Xcode)
version                                              1.4.0-alpha.1
```

For the working tree including the uncommitted work, see §0. **`cargo deny check` now fails**
on a security advisory published after `04f8ddf` (#227) — nothing in this branch caused it.

---

## 2. Verified ground truth

> Confirmed by reading source, running commands, or querying the GitHub API. Where this
> contradicts `README.md`, `PROJECT_STATUS.md` or `docs/changelog.md`, **this file is right**.

### Repository facts

- **9 crate directories**, 8 workspace members. `mm-gtk` is excluded because it needs the
  Linux-only `gettextrs`; root `Cargo.toml` carries `exclude = ["crates/mm-gtk"]` (#199).
- **54,121 Rust lines and 1,467 test functions** in `crates/`, counted on the working tree
  on 2026-09-15, so including the uncommitted work. The 2026-09-06 sweep counted 53,056 and
  1,460 at `04f8ddf`. Figures of 44,183 / 1,392, or of 217 / 399 / 444 tests, are stale.
- **`cargo test --workspace`: 1,395 passed at `83628d9`**, and 1,402 on the working tree.
- **Version `1.4.0-alpha.1`**, the project's first semver pre-release (#214). No tag has been
  pushed for it. `Info.plist` carries `1.4.0`, because Apple's version format cannot hold the
  suffix — which is also why the apps' pre-release warning can never appear (#222).
- **No public release exists.** The only GitHub release is *"MetaMancer v1.0-M1"* (2025-06-16,
  under the project's pre-rename name) and the only tag is `v1.0-M1`. The archive tag
  `v1.5-M6-python-final` does **not** exist (#19). `release.yml` has never run.
- `LICENSE` is tracked and staged into every package (#207).
- **Upstream pin:** `meedya-core` at `222ca7590493` with `features = ["full"]`. The local
  `../MeedyaSuite-core` checkout is about 66 commits ahead, and upstream has since gained 13
  concrete providers and a stems tag model, none of which reaches this build. **Only
  `meedya_core::metadata` and `meedya_core::providers` are actually used here** — the other six
  upstream crates are compiled in and never called.

### What exists, and what does not

| Thing | Reality |
| --- | --- |
| macOS app | Builds. **Does not link the Rust engine** — `MM_FFI_AVAILABLE` is commented out in `Package.swift`. Since `83628d9` it cannot move a file, and says so. |
| Windows app | Never green in CI (#148). Falls back to stubs when `mm_ffi.dll` is missing; the fix for that is in the uncommitted work (§0). |
| Linux GTK app | The only interface running on the real engine. Cannot be compiled on this machine (no GTK4). Moves files without taking the write lock (#226). |
| iPhone, iPad, Vision Pro, Apple Watch | **Not planned.** Judged unsuitable on 2026-09-15 (#228 closed). The Apple app is macOS-only. |
| Android app | **Does not exist yet; planned** as a side-loadable phone app (#230) with foldable support (#229). The only MWBM Android app today is iHymns'. |
| Library database | **Does not exist**, here or upstream (#220). `state/mod.rs` only ever counted work; upstream `meedya-db` is a client for a web service and contains no SQL. |
| API or web server | **Does not exist.** Zero `.route(` calls in `mm-server`, zero `.html` files, zero OpenAPI files. |
| Metadata providers | 13 real ones in code. Only MusicBrainz is reachable — through the GTK panel, search only, with no way to apply a result. `meedya lookup` exits 3 (#83). |
| Cloud (M7), database export (M9), media server (M10) | Scaffolding only. |
| Linux packages | Only a `.deb` and a tarball are built. The Flatpak and Snap recipes point at tags, commits and binary names that do not exist (#107). |
| Development machine | Command Line Tools only — **no Xcode** (so `swift test` cannot run), no .NET, no GTK4, no iOS SDK, no Android SDK. |

### Corrections to earlier records

- ~~Test Mode is never enforced~~ — fixed for tag and cover-art writes (#128, `6894105`).
  **But Test Mode has never covered renames:** the renamer contains no Test Mode logic at all.
  Two pieces of shipped app text claimed otherwise; macOS is corrected, Windows is in §0 (#225).
- ~~`scan --execute` can destroy files~~ — fixed for duplicate destinations (#201) and for disc
  images (#219). **But #219's fix reaches only the command line.** The engine bridge the apps use
  lacks it (#223).
- **Issues #165–#191 — this file was wrong.** It said these 27 DJ and tagging issues were
  "correctly closed" after being moved upstream on 2026-05-29. The 2026-09-06 sweep found the
  decision to close them was right, but the **close reason is wrong**: they are marked
  *completed*, and for most of them nothing exists upstream either. The fix is to re-close them
  as *not planned* — not to reopen them. See §14a.
- **The pattern to keep hunting for: modules that are fully written, well tested, and never
  called.** Confirmed so far: the logging system and the username-stripping in logs (#50), the
  disc-image recogniser (#219), the single-instance lock (#49), the companion detector (#48),
  the GTK error dialogs (#92), the credential store (#75), the engine-bridge callbacks (#65), and
  provider settings in `ProviderConfig` (no issue yet).

### GitHub issue state

- **67 open, 149 closed** after #228 and #229 were filed on 2026-09-15.
- Filed since 2026-09-05: #216–#229. Reopened: #48, #49, #180, #182. Closed as fixed: #210,
  #219. Closed as not planned: #74, #80, #104, #106, #146.
- **Open P0s:** #222 (the Windows half) and #19 (the archive tag).
- Labels `platform:ios` and `platform:android` were created on 2026-09-15.

---

## 3. Owner decisions

| Question | Decision | Date |
| --- | --- | --- |
| Branch base | **Merge `main` into the working branch** — one branch, one PR to `alpha`. | 2026-09-02 |
| Closed-but-stubbed issues (M7/M9/M10) | **Reopen them.** Accuracy wins over milestone burndown. | 2026-09-02 |
| OpenAPI / Swagger UI | **Defer** until an HTTP server exists. There is still no API (§2). | 2026-09-02 |
| Merge versus port | **Do the full port now**, keeping every deliverable. | 2026-09-03 |
| Disc images (#217) | The **whole folder moves as one sealed unit**. Naming is tried as cue sheet → fingerprint *offered as a suggestion* → folder name, and **nothing is renamed when unsure**. **Look inside images** to tell audio discs from film discs. | 2026-09-05 |
| Why an ISO was "software" | Recorded as a rule: classify by what a file means to a media library, never by file mechanics. In `.claude/CLAUDE.md` and `AGENTS.md`. | 2026-09-05 |
| Duplicate detection (#182) | Show every group with full quality details, **recommend a keeper and say why, never act without confirmation**, and keep copies with something unique about them. **Ignore silence**, digital or audible, when comparing. | 2026-09-06 |
| Fingerprinting library | Chose to bundle Chromaprint in every installer — **superseded**: upstream `meedya-fingerprint` already does this in pure Rust, so nothing needs bundling. | 2026-09-06 |
| Library database (#220) | Needed. **The database is the source of truth.** | 2026-09-06 |
| Tags written into files | **Always write both** the standard tag, where one exists, **and** a MeedyaMeta namespaced tag, where the format allows — so detail the standard would flatten (a spine photo becoming "Other") is kept. | 2026-09-06 |
| Media Kind and Art Kind (#221) | Extend the existing companion-type list instead of adding a second vocabulary; use the MusicBrainz artwork vocabulary; Static or Animated art type; identify animated art by filename first. | 2026-09-06 |
| Music stems | **Real audio with a role**, not a sidecar, with its own naming — for example `<Artist> - <Title> [<Stem Part/Instrument>]`. | 2026-09-06 |
| DJ sidecar files | **Track-specific** files — VirtualDJ `.vdjstems` and `.vdjedit` — are sidecars and **move and rename with their media**. Library-wide files such as Rekordbox XML are a different problem. Both were in the original brief. | 2026-09-06 |
| Apple mobile platforms | **Not suitable — the Apple app stays macOS-only.** iPhone, iPad, Vision Pro and Apple Watch cannot reach a user's media library as files or run background watch folders, and outside the EU ship only through the App Store. Revisit only as a *companion* app if the media server (#120) is ever built. #228 closed as not planned. | 2026-09-15 |
| Android | **An Android phone app is viable and wanted** (#230), distributed mainly as a direct download, where Google Play's review of "All files access" does not apply. Foldable support (#229) is a layer on top. | 2026-09-15 |
| Pushing to `alpha` | **Never directly** — not even this file. Work reaches `alpha` only through the pull request. | 2026-09-15 |

Standing instructions:

- **Plain English everywhere** — replies, commits, issues, documentation, in-app help.
- Deep analysis and planning → **Fable, strictly one agent at a time, never in parallel**, even
  when that is slower: several running together risk hitting a usage limit part-way and
  stranding all of them. Fall back to Opus if Fable is unavailable, and try Fable again next
  time. Implementation → Sonnet or Haiku; Opus when genuinely complex.
- **Review by a different system from the builder** — Claude builds and Codex reviews, or the
  reverse. If the reviewer cannot run, say so plainly and owe a catch-up review (§0).
- **Check MeedyaSuite-core first** — including whether the feature flag is switched on — before
  calling anything missing.
- An issue sweep must **reopen anything incomplete and update the detail on every issue, open and
  closed**.
- **No PR stacking.** Commit **and push** after each unit of work, to this branch only, and update
  each relevant issue.
- **Surface decisions up front in one batch**, then carry on autonomously.
- Keep this file current **as the work happens**.

---

## 4. The `main` merge and the MusicBrainz port (step 6 — done)

`main` and `alpha` diverged from common ancestor `24325d2`. `main` carried the MeedyaSuite-core
migration (#132/#133/#135, PRs #159–#163) and CI audit fixes (#153/#155/#157); this branch carried
14 MusicBrainz-hardening commits for #198. Six files conflicted because **`main` deleted
MeedyaManager's local provider trait system** (603 lines) in favour of upstream's, while the
MusicBrainz work was written against the local one.

Resolution taken in `58ea003`:

| File | Side taken | Why |
| --- | --- | --- |
| `mm-providers/src/traits.rs` | `main` | Upstream migration is the sanctioned architecture |
| `mm-providers/src/music/mod.rs` | `main` | `main` had already ported all 16 `impl MetadataProvider` blocks |
| `mm-providers/src/identifiers/mod.rs` | `main` | same |
| `mm-providers/src/rate_limiter.rs` | `main` + re-added `shared_host_limiter` | see below |
| `mm-cli/src/commands/scan.rs` | ours | cosmetic (closure param name) |
| `mm-core/src/i18n.rs` | ours | cosmetic; ours has the fuller SAFETY comments |

`musicbrainz.rs` (1,497 lines, this branch only) was ported to the upstream API:

- `ProviderError::Network(..)` → `NetworkError(..)`
- `ProviderError::RateLimited { provider }` → `RateLimited(..)` (upstream is a **tuple** variant)
- `ProviderResult.provider` → `.provider_name`
- `.provider_id` → `metadata[META_PROVIDER_ID]` **and** upstream's first-class `.musicbrainz_id`
- `.duration_secs` → `metadata[META_DURATION_SECS]`

`governor` was re-added as a direct dependency of `mm-providers` and
`rate_limiter::shared_host_limiter()` restored (both removed by #135). Upstream's
`ProviderRateLimiter` is one-limiter-per-provider and keeps its limiter private, so it **cannot**
express "MusicBrainz, ISRC and ISWC share one bucket". Without it those three each get an
independent 60 RPM bucket and collectively hit musicbrainz.org at up to 180 RPM — 3× the
documented limit. The wiremock test `mb_get_concurrent_calls_share_one_rate_limit_bucket` proves
the shared bucket still works.

### Known gap — pagination

Upstream `SearchQuery` has **no `offset` field** (nor `query`/`country`); it has
`title, artist, album, year, media_type, max_results: Option<usize>, isrc, upc, iswc, eidr,
musicbrainz_id`. So providers cannot pass pagination into `musicbrainz::search_params()`. The
parameter is retained on that function and is still covered by tests. Wiring it needs an upstream
change in MeedyaSuite-core — same territory as open issue #162. Tracked on **#198**.

---

## 5. Session artefacts

The session scratchpad is **session-scoped and will not survive into a new session**:

```text
/private/tmp/claude-501/-Users-lance-manasse-…/07a21012-…/scratchpad/
    review-snapshot.patch    the uncommitted work under review (§0), captured 2026-09-15
    codex-review-2.log       the failed Codex review (usage limit)
    BRIEF.md, audit_*.md, PLAN_proposals.md, ISSUE_ACTIONS.md, comments/, newissues/
                             the 2026-09-02/03 reconciliation audit
```

Records that **do** survive live under `~/.claude/projects/…/07a21012-…/`:

- **The 2026-09-06 issue sweep** — its full report, including a ready-to-post summary for every
  issue, is stored with this session's subagent transcripts in `subagents/`. The verdicts are
  copied into §14a so the follow-up work does not depend on that file surviving.
- Workflow scripts in `workflows/scripts/`, each re-runnable from its run ID.

**Lesson learned:** the *first* issue sweep, on 2026-09-05, ended without ever delivering its
findings, and the work was lost. Conclusions from anything long-running now get copied into this
file as soon as they arrive.

---

## 6. Documentation rewrite scope (step 8)

The docs audit found **121 specific inaccuracies** across **55 `.md` files**. Highlights:

- `docs/issues/github_issues.md` — 119 of 137 titles wrong, 56 issues missing, 6 phantom numbers.
- `docs/issues/milestone_m1.md` — **empty file**.
- `docs/issues/issue_128_accessibility.md` / `issue_130_translation_support.md` — both describe
  issues whose real subjects are different (#128 is Test Mode; #130 is MediaInfo bundling).
- `docs/changelog.md` — entries for v1.3.1/v1.3.2 which were never cut; duplicate v1.1.0.
- `help/rule-syntax.md` — 4 phantom functions documented, 8 real ones undocumented. The real set
  is the 24 in `crates/mm-core/src/rule_engine/functions.rs:120-153`.
- `help/cli-reference.md` — most flag tables wrong.
- `locales/TRANSLATORS.md` — describes a `t!()` macro that does not exist; **zero** `gettext()`
  call sites exist anywhere, and no `.mo` is compiled.
- macOS `.xcstrings` (32 keys) referenced by **0** Swift files; Windows `.resw` (55 entries)
  referenced at 1 site.
- `.env.example` ships unprefixed names (`SPOTIFY_CLIENT_ID`, `METAMANCER_*`) that the code never
  reads — code reads `MM_`-prefixed names only.

---

## 7. If you are resuming cold

1. Read **§0** first, then `.claude/CLAUDE.md` for the project rules, and `AGENTS.md`.
2. Run `git status` and `git log --oneline -5`. **Expect uncommitted work** unless §0 says it has
   been committed. Do not discard it, and do not commit it without a review.
3. Run `export PATH="$HOME/.cargo/bin:$PATH"` before any cargo command.
4. Check whether the review in §0 came back. If the session was lost before it did, run the
   review again from the description in §0, using a system that built none of the work.
5. **Never test `meedya watch --organize` or `meedya service install` against real settings.**
   Point `MM_CONFIG_DIR` at an empty scratch folder and pass `--dry-run`. `service install`
   really does register a background service.
6. If Codex is available again — after **2026-09-20 16:30** — run the catch-up review before
   anything else. Invoke it with `< /dev/null`, or it waits forever for input.
7. Then work down §8.

---

## 8. Next actions

In order:

1. **Act on the stand-in review (§0).** In order: commit the **release guard** with finding #8
   fixed; fix the **Windows guard**'s false Test Mode promise (#5) and commit it; fix the committed
   `scan` defects in #231 in their own commit; **redesign the write lock** around an
   operating-system file lock, which is released even when a process crashes (#6, #7, #16); and
   fix the **organiser**'s critical and high findings (#1–#4) before committing #180. Consider
   keeping `watch --organize` and `service install` refusing until Codex has reviewed them. Every
   commit message must say who reviewed it.
2. **#227** — `cargo update -p rustls` to 0.23.45 or later, alone in its own commit, then confirm
   `cargo deny check` is clean. Upstream (0.23.40) and MeedyaDL (0.23.42) are affected too.
3. **Codex catch-up review** once it is available (after 2026-09-20 16:30), covering everything
   in §0 and every commit listed there as unreviewed.
4. **Act on the sweep (§14a):** post an updated comment on every issue, open and closed; reopen
   the "closed but wrong" list; re-close #165–#191 with the correct reason; close #136; resolve
   #202; file the gaps that have no issue.
5. **#223** — put the disc-image protection somewhere every caller gets it. This blocks #66, #228
   and #229.
6. **#217 stages 4–9** — rule tags, the whole-folder move, looking inside images, the other
   container formats, UDF, and the documentation. The plan is recorded on the issue.
7. **#220, then #221, then #182** — the library database, the classification vocabulary, then
   duplicate detection. Each depends on the one before.
8. The thorough documentation pass and a fresh set of ranked new-work proposals — both requested
   and not yet done.
9. **Open the PR to `alpha`** when the owner says. It will be the branch's first-ever CI run.

### Decisions the owner still needs to make

| Issue | Question | Recommendation |
| --- | --- | --- |
| #225 | Should Test Mode protect renames? | Not yet: keep refusing to rename while Test Mode is on; build rename *undo* on #220's history, then revisit. Full reasoning on #225, including the hard-link and clone counter-argument |
| #230 | Distribution channel? Register MWBM Partners Ltd as a verified Android developer? First-version scope? | Direct download first; register before Google's 2027 worldwide rollout; tags, previews and renames first, with periodic checks instead of instant watch folders |
| #229 | File-access model | "All files access" for the direct-download build; user-picked folders only if a Google Play build is ever wanted. Target current Android; `minSdk 26` like iHymns |
| #19 | Create the Python archive tag, or close as not planned? | Close as not planned unless the tag is actually wanted |
| #215 | Successors for the mirror issues #9–#17 | Decide per issue — #9 and #10 need none |

---

## 9. Branch consolidation (2026-09-03)

The repository had six work-in-progress branches. `alpha`, `beta` and `main` were all already
**fully contained** in this branch after the `58ea003` merge, so the audit reduced to four.

| Branch | Tip SHA | Verdict | Action taken |
| --- | --- | --- | --- |
| `claude/issue-196-identifier-convergence` | `2c1f184` | **Real unmerged work** | Merged (`437f286`) |
| `chore/claude-config-recovery-2026-07-20` | `e05ca2d` | Useful `.claude/` config | Cherry-picked (`b42e8b4`) |
| `feature/134-mm-core-metadata-migration` | `4e67f2a` | Only a settings.json tweak | Cherry-picked (`b42e8b4`) |
| `feature/MeedyaManager_MeedyaSuite-core_integration` | `7254932` | **Superseded entirely** | Nothing to take |

**Recovery:** every SHA above is recorded here deliberately. If a deleted branch is ever needed,
`git fetch origin <sha>` / `git branch <name> <sha>` restores it as long as the object survives
GitHub's reflog retention. Push a tag immediately if you need it permanently.

### Why each verdict

- **`claude/issue-196-identifier-convergence`** — the only branch carrying unmerged production
  code, and the reason #196 was reopened. It converges the eight `META_*` constants onto
  MeedyaSuite-core's canonical unprefixed `extra_keys` names, adds `read_meta()` +
  `LEGACY_META_PREFIX` as a one-release read-both shim, adds a drift-guard test, adds the `iswc`
  entry to `config/tags.json5`, and fixes the `manual_assert_eq` pair that was making the clippy
  gate red (#197). It merged with **only two conflicts, both documentation** — no code conflicts —
  because last session's rewiring touched `musicbrainz.rs`, `rate_limiter.rs`, `music/mod.rs` and
  `identifiers/mod.rs`, while #196 touches `traits.rs`.
- **`feature/MeedyaManager_MeedyaSuite-core_integration`** — its two commits are an *earlier draft*
  of work that later landed on `main` properly via PR #159 (`0c2a366`) and the deny.toml fixes
  #149/#151/#153. Merging it would have been actively harmful: a two-dot diff showed it reverting
  ~11,500 lines, deleting `musicbrainz.rs`, `pr-gate.yml`, this handoff file and the whole
  documentation rewrite, and rolling `deny.toml` back to a pre-v2 schema.
- **`feature/134-mm-core-metadata-migration`** — the name is misleading. Its metadata migration was
  already merged to `main` via PR #163; the only thing left on the branch was a two-line
  `.claude/settings.json` permissions addition.
- **`chore/claude-config-recovery-2026-07-20`** — restored the two subagent definitions and the
  `dev-team` plugin enablement. **`.claude/settings.local.json` was deliberately excluded**: it is
  per-developer machine config and the branch's copy had accumulated permissions for an unrelated
  Python-era project. It is now gitignored so it cannot be committed by accident.
- The recovered agent definitions specified `model: claude-fable-5`, which is **not a valid model
  id** (Fable 5.1 is `claude-fable-5-1`), so they were corrected to the aliases `fable` and
  `haiku`. As written they would not have resolved.

## 10. Round 1 — alpha-readiness work packages (complete)

Planned by a Fable deep-planning pass on 2026-09-03 against tip `3eefc1d`. All seven candidate
issues were re-verified as still real on the current tree. Packages are **file-disjoint by
construction** so implementers can run in parallel; a merge conflict means an implementer strayed
outside its file list and the merge should be rejected.

**Hard rule for every package: no new dependencies.** `Cargo.lock` is the one file every worktree
would collide on.

| Pkg | Closes | Wave | Model | Files (exclusive) |
| --- | --- | --- | --- | --- |
| **G** `P0-CONFIGDIR` | #212 | 0 | sonnet | `mm-core` config/state/health/test_mode/integrity/filetype_registry/tag_registry/settings_bundle, `mm-gtk` app+settings_panel, `mm-ffi` config lines |
| **E** `P0-LICENSE` | #207 | 0 | haiku | `LICENSE`, `release.yml`, `linux/deb`, `linux/snap`, `linux/appimage` |
| **A** `P1-SCAN` | #201, scan half of #206 | 1 | opus | `mm-cli` scan.rs, `mm-core` renamer.rs + evaluator.rs, `mm-gtk` scan_panel |
| **B** `P1-TESTMODE` | #128, edit half of #206 | 1 | opus | `mm-core` integrity/test_mode/metadata, `mm-cli` edit.rs, `mm-gtk` metadata_panel, `mm-ffi` |
| **C** `P1-STUBS-CLI` | #205 CLI half, export DSN of #206 | 1 | sonnet | `mm-cli` export/serve/lookup/main/output |
| **F** `P1-SETTINGS` | #211 | 1 | sonnet | `config/settings.json5` + schema, `mm-core` config/mod.rs, 2 help pages |
| **D** `P1-STUBS-UI` | #205 UI half | 1 | sonnet | GTK/macOS/Windows Server+Export+Cloud views |
| **DOCS** | — | 2 | sonnet | changelog, PROJECT_STATUS, help pages, this file |

**Why G goes first, alone:** it owns files B and F later need, and it introduces `MM_CONFIG_DIR` —
the test-isolation primitive without which B's regression tests would write to the developer's real
config directory. There is currently **no such hook**, and `dirs` 6.0 ignores `XDG_CONFIG_HOME` on
macOS, so `integrity.rs:128` reads the real manifest during `cargo test` today.

### Two findings worse than the filed issues

- **#201 is wider than reported.** `mm-ffi/src/uniffi_api.rs:144-170` and
  `mm-gtk/src/ui/scan_panel.rs:353` both call raw `std::fs::rename`, bypassing `execute_rename`
  entirely — a CLI-only fix protects nothing else. Separately there is a **path-traversal hole**:
  `renamer/mod.rs:222` and `:307` do `output_dir.join(parent_parts)` with unsanitised template
  directory components, and `evaluate_template` does not strip separators. An artist tag of
  `/tmp/x` makes `Path::join` *replace* `output_dir`; `../x` escapes it. This is in the API that
  mm-ffi and mm-gtk already call.
- **#212 is 14 sites, not 6**, spread across three crates.

Also noted: `simulate_rename_with_rules` (`renamer/mod.rs:260-338`) already does intra-batch
conflict tracking and directory splitting correctly but has **zero callers and zero tests**.

### Decisions taken for Round 1 (reversible; flagged to the owner)

1. **#212** — standardise on uppercase `MeedyaManager`; **no migration code**. Never released
   (only tag is the Python-era `v1.0-M1`), and macOS APFS is case-insensitive so both names already
   resolve to one directory. Only a Linux pre-alpha developer could hold a stale
   `~/.config/meedyamanager/`; handled with a changelog line.
2. **`MM_CONFIG_DIR`** — new user-facing override, adopted. Doubles as the test-isolation primitive
   and as a useful "run the alpha in a sandbox" instruction for testers.
3. **#201 `conflict_strategy`** — implement `skip` and `rename` (counter suffix). Treat
   `overwrite` and `ask` as warn-and-skip this round: `overwrite` means deliberately re-enabling
   the data-loss path, and `ask` needs a confirm prompt that does not exist yet.
4. **#205** — add `ExitCode::NOT_IMPLEMENTED = 3` so scripts can distinguish "not built" from
   "failed" (1) and "partial" (2). Keep the commands visible in `--help`, because
   `--show-schema`, `--check-config` and `--show-routes` all do real work.
5. **#128 corrupt manifest** — `is_enabled()` fails **open** (returns `false`) with an `error!`
   log, not closed. Failing closed would make `enable()`/`disable()` error out too, locking the
   user out of Test Mode until they delete the file by hand.
6. **#206 strictness at the core** — `write_tags`/`remove_tag` reject unmapped keys, rather than
   validating only in the CLI, because the FFI path lies identically today.

### Local verification limits (record in every package report)

`mm-gtk` is outside the workspace and needs GTK4/pkg-config, which is **not installed** here.
Xcode is not installed (Command Line Tools only). Windows cannot be built. CI on the eventual PR to
`alpha` is the authority for those files — and per §1 that will be the **first CI run ever** on
this branch (#204).

## 10b. Round 1 outcomes (complete, 2026-09-03)

All seven candidate issues closed: **#128, #201, #205, #206, #207, #211, #212**.
Gate at the end of the round: `cargo test --workspace` **1,303 passed / 0 failed**, `cargo fmt
--all --check` clean, clippy clean excluding the known `mm-cloud` debt (#200), `Cargo.lock`
untouched throughout.

### Three bugs nobody had filed, found by packages working on something adjacent

1. **Path traversal from tag data** (`renamer/mod.rs`) — unsanitised template directory components
   let a tag value escape the output directory; the regression test wrote to `/etc`. Reachable via
   the API `mm-ffi` and `mm-gtk` already call.
2. **`write_tags_safe` had never worked on a real file** — `temp_path` produced
   `track.mp3.meedya_tmp`, and lofty resolves format from the extension alone with no
   content-sniffing fallback, so it failed `UnknownFormat`. Unnoticed because nothing called it and
   its own tests only covered failure paths. Routing `edit.rs` through it, which is precisely what
   #128 asked for, would have broken **every ordinary tag edit**.
3. **A pre-existing flaky test** — twelve tests in `config/mod.rs` touch the process environment
   without synchronisation, failing roughly 1 run in 3 on the untouched baseline. The subtle half
   is that the env-*reading* tests race too: `load_from` applies `MM_*` overrides, so they are
   affected without ever calling `set_var`.

### Where the package specifications were wrong (implementers were right to override)

- `cargo fmt --all` in a shared tree would have reformatted other packages' in-flight files.
- `git stash` for fail-first proofs would have swallowed four other packages' uncommitted work.
  Write the failing tests against untouched production code instead.
- The prescribed `for<'p> Fn(&'p Path) -> MmResult<EvalContext<'p>>` bound is **unsatisfiable** —
  an HRTB quantifies over every lifetime including `'static`, and covariance does not rescue it.
  Use a single named lifetime.
- A tag containing `/` yields **nested directories**, not an underscore: by the time the renamer
  sees the evaluated template, a separator from tag data is indistinguishable from a template one.

**Lesson for future rounds:** parallel packages in one shared working tree must never run
workspace-wide formatters or `git stash`. Scope every tool invocation to the package's own files.

## 10c. Round 2 outcomes — release readiness (complete, 2026-09-03)

Closed: **#197, #200, #202, #203, #204, #214**. Commits `79c4be3`, `2f70850`, `e1958e9`, `9beb3ad`,
`9b7c96c`, `9f3719b`.

### What the packages found beyond the filed issues

- **#202 `release.yml` copied binaries that do not exist.** The CLI binary is `meedya`, not
  `mm-cli` (wrong at seven sites), and `meedya-gtk` was never built at all because `mm-gtk` is
  excluded from the workspace. With 25 `2>/dev/null || true` suppressions, every archive it produced
  would have contained **only `LICENSE`**. Now built explicitly via `--manifest-path`, suppressions
  removed, `.dmg`/`.deb`/`.zip` added to the artefact filter, and a `workflow_dispatch` with
  `publish: false` added so the pipeline can be exercised without cutting a release.
- **#197's diagnosis was wrong.** Both lints that broke the gate — `manual_assert_eq` and
  `unused_async_trait_impl` — are **pedantic**, not nursery, verified with `clippy-driver -Whelp`.
  Dropping nursery would have prevented neither. The toolchain is now pinned to `1.98.0`; the
  eleven `dtolnay/rust-toolchain@stable` usages are deliberately untouched because rustup's
  `find_override_config` puts a directory `rust-toolchain.toml` ahead of `rustup default`.
- **A hard-rule violation nobody had noticed:** `ci-rust.yml`'s `push.paths` omitted
  `rust-toolchain.toml`, `deny.toml`, `clippy.toml` and `.cargo/`, all of which `pr-gate.yml`'s
  detection covered. A direct push touching only the toolchain pin would never have triggered
  Rust CI.
- **Two `deny.toml` ignores were dead entries** — cargo-deny reported `advisory-not-detected`,
  because `unmaintained = "workspace"` skips transitive crates.

### #214 — the version bump was not a find-and-replace

`1.4.0-alpha.1` is **not expressible** on two platforms: MSIX needs four numeric parts and
CFBundleShortVersionString needs three integers. Both therefore carry `1.4.0.0` / `1.4.0` for the
alpha *and* for the eventual final — **MSIX will refuse to install the final over the alpha.**
No MSIX package exists yet (#148, #202), so this is a follow-up before the first Windows package,
not a blocker. Debian gets `1.4.0~alpha.1`: under Debian ordering `~` sorts *before* the release,
whereas `-` would sort *after* it and make the alpha look newer.

Carrier list is now five files, all CI-checked: `Cargo.toml`, `crates/mm-gtk/Cargo.toml` (exact,
suffix included — it cannot inherit, being outside the workspace), `Info.plist` (suffix stripped),
`Package.appxmanifest` (suffix stripped), `linux/snap/snapcraft.yaml` (exact — it had drifted to
`0.9.0`).

### Two known unknowns going into any PR

1. **`mm-gtk` has never been compiled.** Eight files were edited across Round 1 and two more in
   Round 2; there is no GTK4/pkg-config on this machine. Expect the Linux job to be the most likely
   red one, and plan a fix-forward wave rather than treating it as a failure.
2. **Windows CI has never been green** (#148). The release workflow's Windows jobs are
   `continue-on-error` and excluded from the release gate so they cannot block a macOS/Linux alpha.

## 10d. Branch alignment — analysis and plan (2026-09-03)

GitHub showed `alpha` as **57 behind / 2 ahead** of `main`, which looks like it needs reconciling.
It does not: **the working branch is already a strict superset of `main`, `alpha` and `beta`.**

| Branch | vs `main` | Contained in the working branch? |
| --- | --- | --- |
| `claude/musicbrainz-api-migration-7jxszn` | 48 ahead / 0 behind | — |
| `alpha` | 2 ahead / 57 behind | **yes**, entirely |
| `beta` | 0 ahead / 57 behind | **yes** — a strict ancestor of `main` |
| `main` | — | **yes**, entirely |

`alpha`'s two "ahead" commits are `afb0525` + its merge `b50aa90` (the actionlint work, PR #192).
Both are already in the working branch, and `.github/workflows/lint.yml` is byte-identical to
`alpha`'s copy. **So no cherry-pick is needed** — and doing one would create duplicate commits that
conflict when the working branch merges.

Merge characteristics, verified:

- working branch → `main` is a **fast-forward**. `main` is a direct ancestor, so no merge commit
  and no possible conflict.
- working branch → `alpha` is also a fast-forward.
- `main` → `alpha` is **not** — hence the 2-ahead. That resolves itself once the working branch
  lands, because the working branch contains both.

### Branch protection (from the rulesets, not assumed)

- `Keep core branches` (14829073): deletion protection only, on `main`, `alpha`, `beta`.
- `Protect main branch` (14829223): `main` only — deletion, non-fast-forward, **pull_request** and
  **required_status_checks**.

So `main` is the only branch that actually enforces a gate. `alpha` and `beta` can be moved
directly. There is no documented branching model anywhere in the repo (grep found none), and `main`
has not moved in three months while all real work happened elsewhere — so `main` is the de-facto
trunk, and `alpha`/`beta` are best treated as release channels cut *from* it rather than long-lived
divergent branches.

### Recommended sequence

1. **One PR: working branch → `main`.** It is a fast-forward, and `main` is the only branch with
   required checks — the strongest available gate for 48 commits of change.
2. **After it merges, fast-forward `alpha` and `beta` from `main`** (`git push origin main:alpha`
   and `main:beta`). Both then sit at exactly zero divergence.
3. **Do not align `alpha`/`beta` before that** — they would go 48 behind again the moment the PR
   lands, so it is wasted work.

### The one real content difference

`macos/MeedyaManagerTests/AccessibilityTests.swift` (201 lines, 10 `@Test` cases) exists on `alpha`
and `beta` but not on `main`. Deleted deliberately by `d68f36b` during the #148 CI firefighting.

It has not compiled since `cd7944d` regardless: it uses `@testable import MeedyaManager`, and
SwiftPM cannot `@testable import` an **executable** target. That is structural, not a runner
problem — restoring it as-is would fail on `macos-26` exactly as it did on `macos-15`.
`CloudModelTests.swift` already works around the same limitation by declaring a standalone copy of
the model under test; the same pattern would restore all ten tests. Recorded on #146.

Consequence: fast-forwarding `alpha` to `main` drops this file from `alpha` too. Nothing working is
lost, but the accessibility coverage it represented is genuinely gone and no issue tracks restoring
it.

## 11. Change log for this handoff file

- **2026-09-15 (midday)** — the sweep refresh started, run by Opus because Fable was out of credits a
  fourth time. #222's Windows fix is recorded as committed (`eb2c2a9`). The "what to do next" section
  is rewritten to match the work actually in flight.
- **2026-09-15 (morning, later)** — stage (b), the lock redesign, committed as `599d921` after review, fix-ups and a
  proof on its own. #49 stays open for the help-page rewrite.
- **2026-09-15 (morning)** — stage (a) review: a crash on non-Latin names and a library-freezing
  stray cue block commit 1; the rebuild with fixes has started. Filed #236 (name length
  in bytes against characters).
- **2026-09-15 (small hours, later)** — the audit now also checks the Linux app's own lock file
  (`210e942`). It passes `--config deny.toml` explicitly, because cargo-deny finds its policy relative to
  the working folder, not the manifest. Reviewed by Sonnet, whose one finding is fixed in the commit.
- **2026-09-15 (small hours)** — stage (b) review: the locking is sound; the misleading lock-unavailable
  advice and the delete-the-lock help text must be fixed before committing. Sonnet fixer started.
  Filed #235 (Test Mode commit and revert take no lock). Also committed: the `.claude/worktrees/`
  exclusion (`1138c6f`) and the Linux app's `rustls` update (`6a9d757`).
- **2026-09-15 (late night, later)** — #233 confirmed to affect the default settings and raised to
  P0; added to the open P0 list. (The example `config/settings.json5` is safe, and briefly looked as
  though it proved the defaults were — it does not.)
- **2026-09-15 (late night)** — the fix plan recorded as §15; filed #233 (lost extensions) and #234
  (minimum Rust version); stages (a) with `rustls`, and (b), started with Sonnet builders.
- **2026-09-15 (night)** — release guard committed (`b694d70`) and Windows guard committed (`eb2c2a9`), both after
  review plus an orchestrator line-by-line check of the fixes. Fable failed again for planning; Opus
  is planning the lock and organiser fixes.
- **2026-09-15 (evening, later)** — the stand-in review's serious findings were verified; filed
  #231 (committed `scan` defect) and #232 (Windows settings erase); §8 item 1 now lists the fix order.
- **2026-09-15 (evening)** — the Opus stand-in review returned: release guard safe; Windows guard,
  write lock and organiser not safe. Verdicts and all 18 findings recorded in §0 before verification,
  so they cannot be lost.
- **2026-09-15 (later still)** — the #180 documentation is written (uncommitted); recorded two
  organiser findings from checking it: the configured output folder is ignored, and partly
  successful runs are not retried.
- **2026-09-15 (later)** — owner decisions on mobile platforms: the Apple app stays macOS-only
  (#228 closed as not planned); a side-loadable Android phone app is planned (#230), with #229's
  file-access recommendation corrected. The full Test Mode reasoning is recorded on #225.
- **2026-09-15** — brought fully up to date after nine days of drift. Added §0 (current state,
  uncommitted work, review status, limits hit) and §14 (the sweep's verdicts, the two P0s, and the
  new platform issues). Rewrote §1, §2, §3, §5, §7 and §8. Corrected two earlier claims: issues
  #165–#191 were closed with the wrong reason, and #219's fix reaches only the command line.
- **2026-09-06** — recorded the issue-sweep specification (§13).
- **2026-09-05** — the Round 3 regression, Round 4, and the disc-image finding (§12).
- **2026-09-03 (Round 2)** — release-readiness round complete: security advisories, CI on
  alpha/beta, pinned toolchain, mm-cloud clippy debt, release pipeline, and the 1.4.0-alpha.1
  version cut. §10c records what the packages found beyond the filed issues.
- **2026-09-03 (Round 1)** — seven alpha-readiness packages complete; §10b records the three bugs
  nobody had filed and the four places the package specifications were wrong.
- **2026-09-03 (later)** — provider rewiring and the full documentation rewrite landed; commit
  table, final verification numbers and next actions recorded. Added the warning that no CI has
  ever run on this branch.
- **2026-09-03** — merge + MusicBrainz port landed (`58ea003`); issue sweep complete (95 comments,
  40 reopens, #201–#215 filed); documentation scope recorded; pagination gap documented.
- **2026-09-02** — created. Recorded branch divergence, the 9-crate/1,392-test inventory, the
  M7/M9/M10 stub findings, the owner decisions, and the pending merge analysis.

---

## 12. Round 3 regression, Round 4, and the disc-image finding (2026-09-05)

### 12a. A regression we shipped, and caught

Round 3 committed the generated macOS binding files under
`macos/MeedyaManager/Bindings/generated/`. The Swift package declares its app target as
`path: "MeedyaManager"` with nothing excluded, so the build tried to compile those generated
files. They depend on a module (`mm_ffiFFI`) that no part of the package provides.

Measured with `swift build`, ignoring the `#Preview` errors that only happen locally because
they need full Xcode rather than the Command Line Tools:

| Commit | Real error lines |
| ------ | ---------------- |
| before the binding files were added | 1 |
| as Round 3 committed it | **841** |
| after the fix (`9e61ee0`) | 1 |

Had this reached CI it would have failed the macOS check on this branch's first ever run, and
the macOS release file too — and it would have looked like a long-standing problem rather than
something introduced a day earlier.

**Correction that matters more than the fix.** This handoff previously said macOS could not be
checked on this machine. **That was wrong, and it is why Round 3 shipped this blind.**
`swift build` works with the Command Line Tools. Filter out `#Preview` / `PreviewsMacros` /
"external macro" lines and the correct baseline is **1** remaining error line.

### 12b. Round 4 — three fixes, all verified offline

- **`2027096` — TheTVDB (#210, closed).** The provider was sending the raw API key as its
  password. TheTVDB requires trading that key for a temporary pass first, so every request
  would have been rejected in production. It now trades the key, remembers the pass for the
  life of the process, and reports a bad key clearly instead of as a vague network error.
- **`7bd4a0f` — file watching (#45, still open).** The setting controlling how long to wait
  before reacting was worked out and then thrown away, so `meedya watch` printed several
  events for a single file save. Events are now grouped properly. The other half of #45 — a
  fallback for network drives where the normal watching method is unreliable — is still not
  built.
- **`db33e85` — logging (#50, still open).** The logging system had **no callers at all**. Worse,
  `meedya report-bug --include-logs` told people to switch on a setting that did nothing. It
  now works, and the code that strips out usernames and home folder paths (written and tested
  long ago, never once used) is finally applied. Log file rotation is still not built.

Tests were run **twice** for the logging change, because installing a logger affects the whole
program and a single passing run would not prove the tests are safe in any order.

### 12c. Disc images are not handled (#217, #218 — new)

The owner asked on 2026-09-05 whether raw, bit-for-bit disc image copies were already managed.
**They are not.** Verified by reading code, not documents:

1. `classify/mod.rs:863` puts `ISO` in the **Archive** group with `ZIP`, `MSI`, `DEB` and `APK`.
   A perfect copy of an Audio CD is treated like an installer package.
2. `companion/mod.rs:130-132` **does** recognise disc images and cue sheets — but the renamer,
   the code that actually moves files, never calls it. Its only real user is the `debug`
   command. Same "written, tested, never called" pattern as the logging bug.
3. **No whole-folder move exists anywhere.** Zero matches across the codebase. Everything works
   one file at a time.
4. `.mdx` and `.cdr` appear nowhere at all.
5. Nothing can tell an Audio CD image apart from a Bluray image.

**The documentation actively claims otherwise, which is worse than saying nothing.**
`README.md:77` promises "Moves subtitles, cover art, and disc images alongside media" and
`help/faq.md:67` repeats it. Both are false today and must be corrected.

**Owner decisions (2026-09-05), now also in `.claude/CLAUDE.md`:**

1. The **whole containing folder moves as one sealed unit**. A rip carries a log, a checksum
   and artwork that only make sense beside the image, so nothing inside is renamed or split off.
2. Naming order: read the **cue sheet** → **disc fingerprint, offered as a suggestion, never
   applied automatically** → **folder name**. If none is confident, **do not rename at all**.
3. **Look inside the image** to tell an audio disc from a film disc, so each follows its own rules.

The fingerprint step is #218, kept separate because it needs the lookup providers reachable
from the command line first (#83) and only helps audio CDs — there is no equivalent for DVD or
Bluray.

### 12d. There is no API and no website, so there is nothing to document yet

Checked on 2026-09-05: **zero** OpenAPI or Swagger files, **zero** HTML files, and **zero**
routes defined in `mm-server` despite it holding 1,956 lines across 4 files. The media server
is scaffolding. Writing API documentation now would document something that does not exist —
precisely the habit this project is trying to break. Proposed instead as deliberate
"design the contract first, then build to it" work, clearly labelled as not yet built.

### 12e. Current gate (verified at `f33e80b`)

    cargo test --workspace     1,354 passed, 0 failed
    cargo clippy -D warnings   clean
    cargo doc -D warnings      passes
    cargo deny check           clean
    actionlint                 clean
    swift build (macOS)        1 error line = correct baseline

Everything is committed and pushed. Nothing is waiting in the working tree.

---

## 13. Queued: full GitHub issue sweep — the specification (owner, 2026-09-06)

> **Status, 2026-09-15:** ✅ the sweep ran on 2026-09-06 and delivered its report — the verdicts
> are in §14a. ⏳ **Acting on it has not started**: no per-issue comments have been posted, and the
> reopen and re-close actions are still to do (§8, item 4). The specification below remains the
> brief for that work.

Not started. Needs a Fable agent, and **Fable runs strictly one at a time** — the owner asked
for this explicitly on 2026-09-06, because several Fable agents running together risk hitting a
usage limit part-way and blocking each other. So this waits until no other Fable agent is
running.

**A first attempt was launched on 2026-09-05 and ended without ever delivering its findings.
That work is lost and must be redone from scratch.**

### What the owner asked for

1. **Sweep every issue, open and closed.**
2. **Reopen any issue found not to be complete.**
3. **Update the detail on every issue — open *and* closed — not only the ones whose state
   changes.** A closed issue that is genuinely finished still gets a comment recording what was
   verified and how, so the record is trustworthy on its own.
4. **Update the project state documents** to match what is actually true.

### The rule that makes or breaks this sweep

**Check MeedyaSuite-core before calling anything missing.** Most shared functionality lives
upstream, not in this repository. MeedyaManager pins `meedya-core` by git revision with
`features = ["full"]`; the local checkout is at `../MeedyaSuite-core`.

Checking the crate is present is **not enough** — check whether the relevant **feature flag** is
switched on. A crate can be compiled in while the part you want sits behind a default-off
switch. That is exactly what happened with the audio-decoding half of the fingerprinting: it
looked absent and was not.

### Verify against code, never against documents

This project's documents and commit messages have repeatedly overstated what is finished. The
specific pattern to hunt for: **a module fully written, well tested, with zero callers.** Known
real cases — the logging system had no callers at all; the code that strips usernames from logs
was tested but never invoked; the module that recognises disc images is never called by the code
that moves files. **A feature nobody can reach is not finished.**

Cite a file and line actually read, or a command actually run with its output. Never infer state
from an issue title, a commit message, or a previous issue comment.

### Already established, so it need not be re-derived (but verify anything relied on)

- **#219 (P0, open):** `meedya scan --execute` splits a `.cue` from its `.bin` and destroys disc
  images. Reproduced against the real binary. Being fixed now by the #217 work.
- **#182 (reopened):** duplicate detection was closed with nothing built here — but the
  fingerprinting itself exists upstream in `meedya-fingerprint`, in pure Rust
  (`rusty-chromaprint` + `symphonia`), needing no C library and no bundling.
- **#48 (closed, wrong):** the companion detector was built and works, but nothing calls it, so
  from a user's point of view the feature does not exist.
- Closed correctly and recently: #210, #201, #205, #211, #128, #203, #204, #197, #200, #202,
  #214, #212, #207, #206. Closed as not planned with reasons recorded: #74, #80, #104, #106,
  #146. Deliberately still open after partial fixes: #45 (polling fallback missing), #50 (log
  rotation missing).
- **There is no API and no website.** Zero routes in `mm-server` despite 1,956 lines, zero HTML
  files, zero OpenAPI files. Any issue implying a working server or web front end is wrong.

### Output wanted

Grouped by verdict: closed correctly / closed but wrong / open and still valid / open but
actually done / open and should be closed as not planned / duplicate or overlapping. Then a
ranked list of the issues whose current state is most misleading, and any gap in the code with
no issue tracking it at all.

---

## 14. What happened between 2026-09-06 and 2026-09-15

### 14a. The issue sweep — verdicts from the 2026-09-06 report

All 207 issues, checked against the code at `04f8ddf` and against the pinned upstream revision.
**None of the actions below has been taken yet unless marked ✓.**

**Closed correctly — keep closed, and post a comment recording what was verified:**
#2, #3, #20–#44, #46, #51–#62, #64, #70–#73, #76, #77, #84, #89, #90, #93, #102, #109, #111,
#112, #119, #127–#129, #132, #133, #135, #137, #141, #143, #145, #149, #151, #153, #155, #157,
#193, #196, #197, #199–#201, #203–#207, #210–#212, #214, #219. The umbrellas #167 and #191 are
closed only as bookkeeping — nearly all of their children were not delivered.
Caveats worth carrying into those comments: #22 is a scaffold that does not link the engine;
#23 and #27 have never been green in CI; #29, #36 and #109 describe a release workflow that has
never run; #33's version check misses the Debian, Flatpak and WinGet files; #135's `full`
feature compiles six upstream crates that nothing uses.

**Closed, but wrong — reopen:**
#11–#17 (the mirror issues; decide their future through #215), #47 (the configurable half of the
filename sanitiser), #48 ✓, #49 ✓, #63, #65, #67, #68, #69, #75, #79, #81, #82, #85, #86, #88,
#91, #103, #107, #180 ✓.

**Closed with the wrong reason — re-close as *not planned* or *moved upstream*, do not reopen:**
#165, #166, #168–#179, #181, #183–#186, #188–#190. (#187 overlaps #78 and should be tracked
there.)

**Open and still valid** (the sweep recorded exactly what is missing now):
#19, #45, #50, #66, #78, #83, #87, #92, #94–#101, #105, #108, #110, #113–#118, #120–#126, #130,
#131, #134, #138–#140, #148, #162, #164, #182, #194, #195, #198, #208, #209, #213, #215–#218,
#220. Three need their descriptions correcting: #45 should also record that `lib.rs:25`
advertises a retry queue that does not exist; #121 reads as though bcrypt was added, and it never
was; #213 should widen to the six unused upstream crates.

**Open, but actually done — close:** #136 (the upstream contribution happened; file a follow-up
to bump the pin, use the upstream providers, and delete the 9,711-line local copy) and #202 (the
code is in; close after a dry run, or on the code evidence).

**Duplicates and overlaps:** #1 and #4–#8 are Python-era and should be relabelled but left
closed; #9 and #10 were delivered as a JSON5 tag registry and need no successor; #18 duplicates
#90; #187 overlaps #78.

**Most misleading, worst first:** #180; #67, #88 and #63 (with #68 and #69 on Windows); #107;
the DJ and tagging issues above; #75, #49, #65 and #47; #79, #81, #82 and #13; #121; #103; #91,
#85 and #86; #11, #16, #139 and #179.

**Gaps that had no issue when the sweep ran:**

| Gap | Where it stands now |
| --- | --- |
| The background service ran a command that exits "not implemented" | #180 reopened; fix uncommitted |
| The macOS and Windows apps moved files from fabricated previews | #222; macOS fixed, Windows uncommitted |
| `ProviderConfig` settings are read and never used to build a provider | **no issue yet** |
| A watcher retry queue is advertised and does not exist | **no issue yet** — add to #45 |
| The Snap recipe uses the wrong build command and wrong binary names | **no issue yet** — relates to #107 |
| Two cue-sheet parsers: local `disc/cue.rs`, and upstream `cuesheet.rs`, unused | **no issue yet** — note on #217 |
| The macOS and Windows settings screens write their own JSON, never checked against `settings.json5` | **no issue yet** |
| The upstream pin has drifted; providers and tag models now exist upstream and are duplicated here | **no issue yet** — follow-up to #136 |
| `AppState` and `LockFile` were never used | #49; fix uncommitted |

**What the sweep contradicted:** #202 is still open on GitHub; the §10 note that
`simulate_rename_with_rules` has no callers is stale; `.claude/CLAUDE.md` overstated the Linux
packages built; `PROJECT_STATUS.md` overstates M4–M6 on macOS and Windows; line and test counts
had drifted. **What it could not verify:** anything needing GTK4, Xcode or Windows.

### 14b. Found and fixed

- **#219 (P0)** — `scan --execute` split a `.cue` from its `.bin` with an ordinary "organise by
  type" template. Reproduced on the real binary, fixed in `04f8ddf`, and re-proven byte for byte.
- **#222 (P0)** — the macOS app renamed up to 50 files to "[Preview] …" from fabricated previews.
  Fixed in `83628d9` with four independent layers; no Swift code can move a file any more. The
  Windows half is in the uncommitted work.

### 14c. Issues filed, and why each exists

| # | What |
| --- | --- |
| #217 | Disc images as media — stages 1–3 done |
| #218 | Identify an audio disc from its track layout, offered as a suggestion |
| #220 | The library database |
| #221 | What a file is for — Media Kind, Art Kind, stems, DJ sidecars |
| #222 | P0: the apps renamed real files from fabricated previews |
| #223 | The engine bridge lacks the disc-image protection — blocks #66, #228 and #229 |
| #224 | The "rename" conflict strategy renumbers correctly filed files on every run |
| #225 | Decide whether Test Mode should cover renames |
| #226 | The Linux app moves files without the write lock |
| #227 | A `rustls` security advisory fails the dependency audit |
| #228 | iPhone Duo support — **closed as not planned** on 2026-09-15; the Apple app stays macOS-only |
| #229 | Foldable Android support — the layer on top of #230 |
| #231 | `scan` moves half-downloaded files, and a `.cue` whose `.bin` has not arrived — blocks #180 |
| #232 | Windows Settings "Save" erases `settings.json5` down to five settings |
| #230 | Android phone app — side-loadable; the base for #229 |

### 14d. iPhone Duo and foldable Android — what matters for planning

> **Updated later on 2026-09-15 — read this first.** The owner asked whether Apple's mobile platforms
> suit MeedyaManager at all. **They do not**, so #228 is closed and the Apple app stays macOS-only.
> An **Android phone app** is planned instead (#230). Side-loading changes the file-access picture:
> a directly distributed app is not subject to Google Play's review of "All files access", so the
> engine can use real file paths unchanged — #229's recommendation was corrected accordingly. Google's
> developer verification (from 30 September 2026 in four countries, worldwide from 2027) means
> MWBM Partners Ltd must register as a verified developer. Since Android 15, the background service
> types suited to watch folders may run only 6 hours per day, so Android gets periodic checks.
> Parts of the bullets below that discuss iPhone are kept only as history.

The detail is on #228 and #229.

- **Neither app exists**, and **this machine can build neither**: Command Line Tools only (no
  Xcode, iOS SDK or simulators), and no Android SDK, NDK, Android Studio or Rust Android targets.
- **iPhone Duo** goes on sale on 23 October 2026, running iOS 27.1 (Apple Newsroom). Apple's tech
  talk confirms that the open screen reports "regular" size classes in both directions and ignores
  orientation locks. Xcode 27.1 and Apple's preparation guide are "coming later this month". **The
  names of the fold-event APIs are not confirmed by Apple yet.**
- **Android 17** already makes adaptive layouts compulsory for apps targeting API 37 on screens
  600dp or wider, with no opt-out.
- **File access decides feasibility on both.** An iPhone app only reaches folders the user hands
  over. Android's equivalent route gives `content://` addresses, which the Rust engine cannot use
  as it stands.
- **The engine bridge already builds as a shared library and uses UniFFI**, which can generate
  Kotlin as well as Swift.
- **Other MWBM apps:** MeedyaConverter#500 and iHymns#2117 track the same device. iHymns has the
  only MWBM Android app, so it sets the Android conventions.

### 14e. Limits hit, and what was tried and rejected

**Limits:**

- **Claude's monthly spend limit** stopped the `watch --organize` agent part-way through, shortly
  after 2026-09-07. It left compiling, tested code with no report and no documentation.
- **Codex's usage limit** blocks the usual independent review until **2026-09-20 16:30**.
- **Fable was out of usage credits** on 2026-09-15, four times in all. The third was the lock
  redesign's planning. The fourth was the sweep refresh at midday, which Opus took over. The first two: the first stand-in review failed the
  moment it started, and so did the planning run for the fixes. Opus stood in instead — independent of the Sonnet-built work, but the same model as
  the builder of the write lock and the organiser.

**Tried and rejected — do not repeat these:**

- **Pushing this file straight to `alpha`.** The owner reversed it, and it would have caused a
  merge conflict when the pull request lands.
- **Running `codex exec` without `< /dev/null`.** It hangs forever on "Reading additional input
  from stdin...".
- **`grep -c` inside a chain of `&&` commands.** A count of zero counts as failure and silently
  stops everything after it. Add `|| true`.
- **Running Fable agents in parallel.** Forbidden by the owner.
- **Bundling the Chromaprint C library.** Unnecessary — upstream does it in pure Rust.
- **Wiring `AppState` in.** Deleted instead: it only counted work, and the database will own
  history.
- **Linking the macOS engine now.** It would bring #219's data loss back through #223, and Test
  Mode would still not cover renames.
- **Deciding whether a video is animated artwork by its length.** A short film looks the same;
  the filename comes first.
- **A plain text search for `moveItem` as proof of safety.** It matches comments as well as code;
  the real proof is zero actual calls.
- **Reading Apple's *Designing for iPhone Duo* guidelines page.** Its content could not be read on
  2026-09-15.

---

## 15. The fix plan for #231, #233, #49 and #180 (2026-09-15)

Planned by **Opus standing in for Fable** (out of credits), after the stand-in review in §0. The full
report is in this session's subagent transcripts. What follows is the plan as adopted — enough to carry
on without that report.

### Rules for carrying it out

- **Order:**
  - (a) `scan` defects
  - (b) the lock
  - (c) the organiser's critical findings
  - (d) the start-up sweep, Ctrl+C and service honesty
  - (e) the remaining findings
  - (f) documentation
  - (g) switch organising on
- **`watch --organize` (except with `--dry-run`) and `service install` keep refusing (exit 3) until (g).**
  Stage (g) happens only after Codex reviews (b)–(e) as one body of work, and not before 2026-09-20 16:30.
- **Prove each commit on its own** in a throwaway `git worktree`. The gate run in the working tree
  includes uncommitted work, so it proves nothing about a single commit.
- **Do not commit the uncommitted #49/#180 documentation with the code.** It describes behaviour that
  is being changed.
- **Build with Sonnet, review with Opus.** With Codex and Fable unavailable, that is the only way to
  get a reviewer from a different model.

### (a) `scan` — #231 and #233

Files: `renamer/mod.rs`, `watcher/mod.rs`, `disc/mod.rs` — all unchanged in the working tree.

- **Lost extensions.** One helper, used by both simulate functions, adds the source's ending back unless
  the new name already ends with `.<ending>`, compared case-insensitively. Trade-off: a template that
  writes a different ending gets both.
- **Downloads in progress.** `watcher::is_download_in_progress` covers:
  - the endings `crdownload part partial download opdownload filepart tmp temp aria2 !ut !qb`;
  - files inside a `.download` folder;
  - rsync's `.name.XXXXXX` temporary files;
  - names starting `~$`.

  `should_ignore` ignores these unconditionally. Today `scan` and `watch` pass empty ignore lists, so the
  default list is never used.
- **Discs.**
  - A cue naming disc-image files forms a set even when some files are missing.
  - An `.mds` without its `.mdf` forms a set.
  - **Any file ending in `iso nrg mds mdf mdx cdr bin cue` is never renamed on its own.**
  - The false comments are corrected.
- **Tests that must fail first:**
  - `a_name_containing_a_full_stop_keeps_its_file_ending`
  - `a_title_containing_a_full_stop_keeps_its_file_ending`
  - `a_partial_download_keeps_its_whole_name`
  - `downloads_in_progress_are_ignored_even_with_empty_settings_lists`
  - `a_file_inside_a_safari_download_folder_is_ignored`
  - `a_cue_whose_image_has_not_arrived_still_protects_its_folder`
  - `an_unreadable_cue_sheet_is_never_renamed_on_its_own`
  - `a_cue_indexing_flac_tracks_stays_beside_them`
  - `a_music_bin_without_a_sync_pattern_is_never_renamed_on_its_own`
  - `an_mds_without_its_mdf_protects_its_folder`
- **The `rustls` update (#227)** is built in the same isolated copy, as a separate lock-file-only commit.

### (b) The write lock — #49

- **`std::fs::File::try_lock`** (stable from Rust 1.89; the toolchain is pinned to 1.98), with
  `#[allow(clippy::incompatible_msrv)]` because `rust-version` is 1.85 (#234).
- **Never delete `meedya.lock`**; closing the file releases the lock. A `meedya.lock.info` note
  (`key=value` text, best effort, used for messages only) says who holds it. On Windows the file is
  opened without delete sharing.
- **The operating system releases the lock however the process ends**, including `panic = "abort"` and
  kill -9. It covers only the same settings folder on the same computer. **Deleting the file by hand
  while it is held defeats it on macOS and Linux**, so `help/troubleshooting.md` must stop advising that.
- `is_process_running`, the `unsafe` blocks and `extern crate libc` are deleted.
- **The blocked message:** *"Another copy of MeedyaManager is moving files right now, so nothing has
  been moved. It appears to be process N (…, started HH:MM). Wait for it to finish, then run this
  again."* Exit code 1.
- **This commit also carries #180's small change in `scan.rs`** — `prompt_confirm` taking the message
  text. It sits right beside the lock code, and splitting the two risks a commit that does not build.
- **Tests that must fail first:**
  - `a_lock_file_naming_an_unrelated_live_process_does_not_block`
  - `an_empty_lock_file_does_not_block`
  - `releasing_never_deletes_the_lock_file`
  - `a_holder_never_deletes_a_lock_file_that_was_replaced` (Unix)

  **Regression tests:** 8 threads × 200 rounds, plus child-process tests for a holder that aborts and a
  holder that stays alive.

### (c) The organiser's critical findings (#1, #2)

Files: `renamer`, `scan.rs`, `disc`, `watch.rs`, `context.rs`.

- **The rename loop.**
  - A shared `plan_destination`, plus `destinations_that_would_move_again`, which recalculates each
    planned destination as though the file were already there. Such files are left alone with the status
    *"Left alone: would be renamed again next time"*, a JSON field `repeats`, and exit code 2.
  - `RenamePreview` and `RenameSummary` keep their shape, because `mm-gtk` depends on them.
- **Disc subfolders.** `ScanArgs.protect_within` sets a boundary, and every folder up to it is checked.
- **Partial downloads.** A folder directly holding a download in progress is skipped.
- **Switched off.** `watch --organize` without `--dry-run` refuses with exit code 3.

### (d) The start-up sweep, Ctrl+C and service honesty (#3, #4)

- **The sweep is opt-in** (`--organize-existing`). The prompt counts the files and folders that will
  move; the deepest watched folder owns each file; the service never sweeps.
- **No terminal and no `--yes` means refuse.** This protects services installed without `--yes`.
- **Ctrl+C:**
  - a stop flag is checked between folders;
  - the first press prints *"Stopping after the folder being organised now…"*;
  - a second press exits with code 130 — but first check how moves across drives are carried out;
  - "Watcher stopped" is printed only once it really has stopped.
- **Install is honest:** it registers the service, starts it, and says so. It refuses when no watch
  folders are set. **In this build it refuses outright**; `--dry-run` prints the unit or plist.
- **Separate Windows and non-Windows install helpers**, which also fixes a confirmed `needless_return`
  Clippy failure on Windows.

### (e) The remaining findings (#11–#17)

- **"Settled"** means all of:
  - no events for the settle time;
  - the same file list, with the same sizes and modified times, as a list taken at least one settle time
    earlier;
  - no download in progress;
  - the folder still exists.

  The default settle time is **30 seconds**.
- **A folder moved in whole** is organised recursively once it has settled.
- **A busy lock:** retry with a growing wait, from 30 s up to a cap of 10 min, printing one message per
  busy spell. The sweep retries too.
- **Also:**
  - honour the configured output folder;
  - resolve watch folders and event paths to their real paths;
  - write macOS logs to `~/Library/Logs/MeedyaManager/`;
  - quote the systemd `ExecStart` path and escape the plist;
  - use `Restart=on-abnormal` with a start limit.

### (f) Documentation

Only after the other doc edits are committed. These searches must come back empty:
- advice to delete `meedya.lock`;
- `/tmp/meedyamanager`;
- "set to start at login";
- a prompt that only says "as they arrive".

### (g) Switch organising on

After Codex's review: remove the two refusals.

### What the plan found on top of the review

- Lost extensions in committed code (#233) — **the default settings are affected**, so it was raised
  to P0.
- Subfolders of disc folders unprotected (#231).
- A music CD's `.bin` has no sync pattern (#231).
- `try_lock` fails Clippy under `rust-version` 1.85 (#234).
- Ctrl+C prints "Watcher stopped" while files are still moving.
- Old service definitions lack `--yes` (#180).
- One `.iso` at a library root freezes the library (#217).
- Folders moved in whole are never organised (#180).
- The organiser undoes manual template runs (#180).

### Not now

- Whole-folder moves (#217), fingerprinting, and FLAC-plus-cue images.
- Waiting for an unplugged drive, a sweep inside the service, stopping gracefully on a stop signal, and
  running in the background on Windows.
- Raising `rust-version` (#234), and deciding what to do about a root-level `.iso`.
- The engine bridge's missing disc protection (#223), and the rename counter bug (#224).
- Stopping part-way through one huge folder, and an override for one-off `<Filename>` renames.

### Progress, 2026-09-15

- **(a) and `rustls`: built as two local commits, not yet on the branch.** `b023adf` (stage a) and
  `3739c19` (`rustls` 0.23.37 → 0.23.45, plus `rustls-webpki`, `aws-lc-rs` and `aws-lc-sys`; lock file
  only), in the worktree `.claude/worktrees/agent-ac1c1ad32f50eb48f`, branch
  `worktree-agent-ac1c1ad32f50eb48f`. The builder reported 1,405 tests passing (the 10 new ones), the
  renamer and disc tests shown failing first (the two watcher tests only by construction), `cargo deny`
  clean after commit 2, and a matching hand check. **The worktree was created 74 commits behind the
  branch head; the builder noticed and moved it forward to `4cd8fa0` first.** Now: an Opus review,
  then cherry-pick both onto the branch — after the stage (b) review finishes, because the `rustls`
  change rebuilds dependencies.
  **Review verdict (Opus, reviewing Sonnet's work):**
  - **Commit 1 must not land as built.**
    - **Critical:** the new extension check sliced names by byte position, so names in Japanese, Chinese or
      Russian crash the whole scan (exit code 101), on default settings. The reviewer reproduced this with old
      and new binaries.
    - **High:** a stray `.cue` naming a missing file, or a lone `.mds`, turned a library root into a disc
      folder and froze every rename beneath it.
    - **Lower:** the `~` and Safari skipping rules were too broad; some comments were wrong.
  - **Commit 2 is safe**, but `aws-lc-sys` 0.45 may link a copy of AWS-LC already on the build machine unless
    `AWS_LC_SYS_USE_SYSTEM=0` is set.
  - **Rebuilding both commits** in the same worktree with those fixes: Sonnet builds, then an Opus re-review.
    **The disc fix is deliberately narrower than the reviewer suggested.** An incomplete set protects its
    folder only when no subfolder holds audio or video. "The scan root is never a disc folder" would have
    scattered the log and artwork when a single rip folder is scanned directly.
  - The reviewer's point that the Linux lock file was still vulnerable was already fixed (`6a9d757`, audit
    `210e942`).
  - **An older bug it spotted is now filed (#236):** the name-length limit is
    measured in bytes but applied in characters, so long non-Latin names can still be too long.
- **(b): ✅ committed `599d921`.** Proof on its own: cargo fmt clean; clippy -D warnings clean; cargo test --workspace 1403 passed, 0 failed, run twice (identical); cargo doc clean; cargo deny fails only on RUSTSEC-2026-0285. The history of how it got there follows. Sonnet's builder reported every must-fail test failing on the old
  code and passing on the new; `cargo test --workspace` 1,412 passed twice; clippy and doc clean. Hand
  check on a 150,000-file scratch library: a second copy was refused with the specified message and
  exit 1; after `kill -9` of the first, the next run proceeded; `meedya.lock` still present. The
  `incompatible_msrv` allowance sits on two functions, not one. **Windows lint unverified** — a
  dependency's C build needs a Windows C toolchain. Now: an Opus review, and proving the commit on its
  own in a throwaway worktree.
  **Review verdict (Opus, reviewing Sonnet's work): the locking itself is sound, but not yet safe to commit.**
  - **Blocking, in code:** the "cannot hold a lock" message told users to point `MM_CONFIG_DIR` at another
    folder. That separates the terminal from the app and the service, resets settings to defaults, turns
    Test Mode off, and blames network drives even for a permissions error.
  - **Blocking, in the uncommitted #49 help pages:** they still tell users to delete `meedya.lock`. The
    reviewer proved it: with a live holder running, deleting the file let `scan --execute` move files. Those
    pages stay out of the commit and are rewritten in stage (f).
  - **Minor:** the macOS app's busy message said "or stop it"; some comments were inaccurate (Windows share
    mode, network drives, leftover `libc`/`winapi`); some test details were weak.
  - **A new coverage gap:** Test Mode commit and revert move files without the lock (#235).
  - A Sonnet fixer is working on the code findings. **The worktree proof that was started before the fixes
    is now stale; re-run it after them.**
- **(c)–(g):** not started.
