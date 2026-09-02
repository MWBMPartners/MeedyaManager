# MeedyaManager — Session Handoff

> **(C) 2025-2026 MWBM Partners Ltd**
>
> **Purpose:** this file is the single place to look to resume work after any interruption.
> It records *verified* state only — never aspirational state. Update it after every task.

**Last updated:** 2026-09-02
**Updated by:** Claude Opus 5 (1M context), session `07a21012`
**Working branch:** `claude/musicbrainz-api-migration-7jxszn` → will PR into **`alpha`**

---

## 1. Where we are right now

### Current task (in flight)

A full project-state reconciliation, commissioned 2026-09-02:

1. Audit **all 187 GitHub issues** (17 open / 170 closed) against the **actual codebase** —
   no inference from commit messages, changelogs or docs.
2. Correct the issue record on GitHub.
3. Rewrite the documentation set to match reality.
4. Refresh `.claude/` context + Claude memory.
5. Produce a ranked list of proposed next work for the alpha releases.

### Status board

| # | Step | State |
|---|------|-------|
| 1 | Ground-truth reconnaissance of repo + issues | ✅ done |
| 2 | Owner decisions captured (see §3) | ✅ done |
| 3 | Deep audit — 9 sequential Fable-5 domain agents | 🔄 running (workflow `wf_0fb16a09-0df`) |
| 4 | Merge `main` into working branch (21 conflict hunks) | ⏳ blocked on step 3 |
| 5 | Apply GitHub issue corrections (reopen + comment) | ⏳ blocked on step 3 |
| 6 | Documentation rewrite (all `.md` + `help/`) | ⏳ blocked on step 3 |
| 7 | Refresh `.claude/CLAUDE.md` + Claude memory | ⏳ blocked on step 3 |
| 8 | Ranked new-work proposals to owner | ⏳ blocked on step 3 |

---

## 2. Verified ground truth (established 2026-09-02, read from source)

> Everything in this section was confirmed by reading code or querying the GitHub API.
> Where it contradicts `README.md` / `PROJECT_STATUS.md` / `docs/changelog.md`, **this file is right
> and those files are stale** — correcting them is step 6 above.

### Branch topology — `main` and `alpha` have diverged

```text
                            24325d2  (common ancestor, "fix: resolve CI failures")
                           /        \
   origin/alpha  b50aa90 ←─          ─→ main  aa0ad24
        │        (+2: actionlint)       (+~30: MeedyaSuite-core migration,
        │                                      issues #132/#133/#135, PRs #159-#163,
        │                                      CI audit fixes #153/#155/#157)
        │
        └─→ claude/musicbrainz-api-migration-7jxszn  b0ffe8c   ← WE ARE HERE
             (+14: MusicBrainz hardening, issue #198)
```

- `origin/beta` = `24325d2` (sits on the common ancestor).
- **Neither `alpha` nor this branch contains the MeedyaSuite-core migration work that is on `main`.**

### Cargo workspace — **9** crates, not 8

`mm-gtk` is deliberately excluded from `[workspace] members` (it needs Linux-only `gettextrs`),
but the root `Cargo.toml` has **no `exclude` key** — this is exactly what open issue **#199** reports.

| Crate | Rust LOC | `#[test]` fns |
|---|---:|---:|
| `mm-core` | 14,830 | 512 |
| `mm-providers` | 10,552 | 386 |
| `mm-gtk` | 5,043 | 67 |
| `mm-cli` | 4,517 | 73 |
| `mm-cloud` | 2,580 | 117 |
| `mm-export` | 2,474 | 111 |
| `mm-server` | 1,956 | 74 |
| `mm-ffi` | 1,626 | 23 |
| `mm-update` | 605 | 29 |
| **Total** | **44,183** | **1,392** |

> `.claude/CLAUDE.md` currently says *"8 crates"* and the docs cite *217 / 399 / 444* tests.
> Both are stale. `mm-update` is the undocumented 9th crate.

### Completion reality vs the issue record

- **M10 (Secure media server, #120–#127)** — closed as *"all M10 features implemented and
  verified"*. In fact `mm-server` **never builds an axum router** (no `.route(` call exists
  anywhere in the crate, despite `axum`/`tower`/`tower-http`/`rustls` being declared
  dependencies), and `crates/mm-cli/src/commands/serve.rs:337-342` prints
  *"Server stub: exiting cleanly (full axum server wired in release build)."*
  There are **zero `.html` files in the repository**, so #124 *"Web frontend (embedded static
  files)"* has no deliverable at all.
- **M9 (DB export, #112–#119)** — `sqlx` + `tiberius` are declared, but
  `crates/mm-export/src/sqlite.rs:30` reads *"In production this holds a `sqlx::SqlitePool`; for
  M9 the pool is …"*. Backends look like DSN/schema scaffolds. (Per-backend confirmation pending
  from the audit.)
- **M7 (Cloud, #94–#102)** — `crates/mm-cloud/src/onedrive.rs:90` reads *"In production this parses
  `reqwest::Response` JSON; here it is a stub"*; OAuth flows exist only as comments.
  `icloud.rs:13` and `mega.rs:12` still carry placeholder `issues/TBD` URLs.
- **Issues #165–#191 (27 DJ / tagging issues)** — these are **correctly closed**. They were closed
  deliberately on 2026-06-09: *"Closing per owner direction 2026-05-29 — this work will be tracked
  and developed in MeedyaSuite-core directly. MM-side consumer issues will be refiled once the
  upstream is ready to integrate against."* Do **not** treat these as bogus closures. The only
  nuance is that GitHub records them as `stateReason: COMPLETED` rather than `NOT_PLANNED`.
- Every one of the 170 closed issues is recorded as `stateReason: COMPLETED`.

### Other confirmed facts

- Only **2** `TODO`/`FIXME` markers exist repo-wide — unfinished work is described in prose
  comments instead. Grep for `stub`, `in production`, `placeholder`, `for M9`, `when … is wired`.
- **No OpenAPI / Swagger / utoipa / redoc anywhere** in the repo.
- Claude memory directory was **empty** at session start; there was **no handoff document**
  (this file is new).
- `gh` CLI is authenticated as `Salem874`.

### Open issues (17)

`#130` `#131` `#134` `#136` `#138` `#139` `#140` `#146` `#162` `#164` `#193` `#194` `#195`
`#197` `#198` `#199` `#200`

---

## 3. Owner decisions taken 2026-09-02

| Question | Decision |
|---|---|
| Branch base | **Merge `main` into the working branch first**, so one branch holds everything and one PR goes to `alpha`. |
| Closed-but-stubbed issues (M7/M9/M10) | **Reopen the affected issues.** Accept the milestone burndown regressing — accuracy wins. |
| OpenAPI / Swagger UI | **Defer** until the axum router actually exists. Record it in the proposals list as blocked, do not write a spec for a server that cannot run. |

Standing instructions for this workstream:

- Deep analysis / deep planning → **sequential Fable 5** agents (fall back to Opus per-call, retry
  Fable on the next call). Implementation → Sonnet or Haiku; Opus only if genuinely complex.
- **No PR stacking.** Everything lands on this one branch; the PR to `alpha` comes later.
- Commit **and push** after each unit of work, updating the relevant GitHub issue each time.
- Keep this handoff file current as work proceeds.

---

## 4. The pending `main` → working-branch merge

Dry-run (`git merge-tree --write-tree HEAD main`) gives **21 conflict hunks across 6 files**:

| File | Hunks | Why it conflicts |
|---|---:|---|
| `crates/mm-providers/src/identifiers/mod.rs` | 7 | `main` moved to the upstream provider trait; HEAD extended the local one |
| `crates/mm-providers/src/music/mod.rs` | 6 | same |
| `crates/mm-providers/src/rate_limiter.rs` | 4 | `main` migrated it to `meedya-providers` |
| `crates/mm-core/src/i18n.rs` | 2 | independent edits |
| `crates/mm-providers/src/traits.rs` | 1 | `main` re-exports upstream types; HEAD kept local `META_*` consts |
| `crates/mm-cli/src/commands/scan.rs` | 1 | independent edits |

**This is a semantic merge, not a textual one:** `main` replaced MeedyaManager's local provider
trait system with the upstream `meedya-providers` one, while this branch built new MusicBrainz
functionality *on top of the local trait system*. Resolving it means **porting the MusicBrainz
hardening onto the upstream trait API**, not picking sides hunk by hunk.

Deliberately deferred until the `mm-providers` audit reports the exact API delta between the two.

To reproduce the merge safely without disturbing the main tree:

```bash
git worktree add --detach /tmp/wt-merge HEAD
git -C /tmp/wt-merge merge --no-commit --no-ff main
```

---

## 5. If you are resuming cold

1. Read this file, then `.claude/CLAUDE.md` for the project rules.
2. Check the audit workflow result: transcript dir
   `~/.claude/projects/…/subagents/workflows/wf_0fb16a09-0df` — read `journal.jsonl` first.
   Audit reports are written to the session scratchpad as `audit_<domain>.md`, plus
   `PLAN_proposals.md` and `ISSUE_ACTIONS.md`.
3. If that scratchpad is gone (it is session-scoped and *will* be gone in a new session),
   re-run the audit — the workflow script is saved at
   `~/.claude/projects/…/workflows/scripts/mm-state-audit-wf_0fb16a09-0df.js`.
4. Pick up the status board in §1 at the first ⏳ row.

---

## 6. Change log for this handoff file

- **2026-09-02** — created. Recorded branch divergence, the 9-crate/1,392-test true inventory,
  the M7/M9/M10 stub findings, the three owner decisions, and the pending merge analysis.
