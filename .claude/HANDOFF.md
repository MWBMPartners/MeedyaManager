# MeedyaManager — Session Handoff

> **(C) 2025-2026 MWBM Partners Ltd**
>
> **Purpose:** the single place to look to resume work after any interruption.
> Records *verified* state only — never aspirational state. Update after every task.

**Last updated:** 2026-09-03
**Updated by:** Claude Opus 5 (1M context), session `07a21012`
**Working branch:** `claude/musicbrainz-api-migration-7jxszn` → will PR into **`alpha`**
**Branch HEAD:** `58ea003` (merge commit)

---

## 1. Status board

Commissioned 2026-09-02: full project-state reconciliation — audit all GitHub issues against the
*actual codebase*, correct the record, rewrite the documentation, refresh Claude context, and
propose ranked next work for the alpha releases.

| # | Step | State |
| --- | ------ | ------- |
| 1 | Ground-truth reconnaissance of repo + issues | ✅ done |
| 2 | Owner decisions captured (§3) | ✅ done |
| 3 | Deep audit — 9 sequential Fable-5 domain agents + planning | ✅ done |
| 4 | GitHub issue sweep — 95 comments, 40 reopens, 4 relabels | ✅ done |
| 5 | File follow-up issues from the audit (#201–#215) | ✅ done |
| 6 | Merge `main` into the working branch + port `musicbrainz.rs` | ✅ done (`58ea003`) |
| 7 | Rewire MB/ISRC/ISWC providers through the hardened module | ✅ done (`2b08392`) |
| 8 | Documentation rewrite (51 `.md` files incl. all 19 provider pages) | ✅ done (`a4d45dc`, `b0bb469`, `6f69478`) |
| 9 | Refresh `.claude/CLAUDE.md` + Claude memory | ✅ done |
| 10 | Present ranked new-work proposals to owner | ✅ done (28 proposals, delivered in chat) |
| 11 | **Open the PR to `alpha`** | ⏳ owner's call — not created, per the no-PR-stacking rule |
| 12 | Post-PR dev-cache cleanup (per `.claude/CLAUDE.md`) | ⏳ after the PR exists |

### Commits on this branch (all pushed)

| Commit | Summary |
| --- | --- |
| `939c6af` | `.claude/HANDOFF.md` created |
| `58ea003` | Merge `main`; port `musicbrainz.rs` onto the upstream provider API |
| `2b08392` | Rewire MusicBrainz/ISRC/ISWC through the hardened seam (#198) |
| `a4d45dc` | Documentation rewrite — 32 files, 121 audited inaccuracies |
| `b0bb469` | Test-count corrections + changelog historical caveat |
| `6f69478` | All 19 `help/providers/` pages rewritten |

**Final verification:** `cargo test --workspace` = **1,240 passed, 0 failed**;
`cargo clippy -p mm-core -p mm-providers -p mm-cli --all-targets -- -D warnings` clean;
working tree clean.

> ⚠️ **CI has never run on this branch.** Every workflow triggers on `main` only, so nothing here
> has been through CI — that is issue #204. Expect the first PR to `alpha` to surface failures,
> particularly the Security Audit (#203, red every week since 2026-07-06) and the `mm-cloud`
> clippy debt (#200, exactly 40 errors).

**Not doing (owner decision):** OpenAPI / Swagger UI — deferred until the axum router actually
exists. Do not write a spec for a server that cannot run. See §3.

---

## 2. Verified ground truth

> Confirmed by reading source or querying the GitHub API. Where this contradicts
> `README.md` / `PROJECT_STATUS.md` / `docs/changelog.md`, **this file is right** — correcting
> those is step 8.

### Repository facts

- **9 crate directories**, 8 workspace members (`mm-gtk` excluded — needs Linux-only `gettextrs`).
  `mm-update` is the undocumented 9th crate. Root `Cargo.toml` has **no `exclude` key** — open
  issue #199 (fixed on `main` by `1d2576d`, and that fix is now merged in here).
- **44,183 Rust LOC; 1,392 `#[test]`/`#[tokio::test]` functions.** Docs claiming "8 crates" and
  217/399/444 tests are stale.
- `cargo test --workspace`: **1,240 passed, 0 failed** (1,207 immediately after the merge; the
  provider rewiring in `2b08392` added 33 tests).
- Version is **1.3.0** in `Cargo.toml`, `Info.plist` and `Package.appxmanifest`. Versions 1.3.1
  and 1.3.2 were **never cut** despite changelog entries for them. Linux/WinGet manifests are
  unsynced (snap/deb say `0.9.0`, WinGet `1.0.0`, flatpak has a placeholder commit pin).
- **No public release exists.** The only GitHub release is *"MetaMancer v1.0-M1"* (2025-06-16) —
  the pre-rename project name. Only tag is `v1.0-M1`. The archive tag `v1.5-M6-python-final`
  referenced by `.claude/CLAUDE.md` **does not exist** (this is why #19 was reopened).
- **No `LICENSE` file** is tracked, though GPL-2.0-or-later is declared everywhere (#207).

### Completion reality

- **M10 server (#120–#127)** — `mm-server` never builds an axum router (no `.route(` call exists);
  `crates/mm-cli/src/commands/serve.rs:337-342` prints *"Server stub: exiting cleanly"*. The repo
  has **zero `.html` files**, so #124's web frontend has no deliverable.
- **M9 export (#112–#119)** — `sqlx`/`tiberius` declared; no pool is ever created, no SQL executed.
- **M7 cloud (#94–#102)** — no real network calls; OAuth flows exist only as comments.
- **Test Mode (#128) is never enforced.** `integrity::write_tags_safe` is referenced only by its
  own tests; all three consumers (`mm-cli/src/commands/edit.rs:116`,
  `mm-gtk/src/ui/metadata_panel.rs:313`, `mm-ffi/src/uniffi_api.rs:233`) call
  `metadata::write_tags` directly, which writes to the original file.
- **`meedya scan --execute` can destroy files** — see #201, the highest-severity finding.
  `crates/mm-cli/src/commands/scan.rs:173` computes `conflict = !unchanged && dest.exists()` at
  preview time and never tracks intra-batch duplicate destinations; that stale flag is passed to
  `execute_rename`, whose guard (`crates/mm-core/src/renamer/mod.rs:349`) therefore passes, and
  `std::fs::rename` silently overwrites. mm-core's own `simulate_rename` gets this right
  (`renamer/mod.rs:225`) — the CLI just doesn't use it. Reproduced empirically.
- **Issues #165–#191 (27 DJ/tagging issues) are correctly closed** — deliberately moved to
  MeedyaSuite-core by owner direction on 2026-05-29. Do not treat as bogus closures.
- Only **2** `TODO`/`FIXME` markers repo-wide; unfinished work is described in prose. Grep for
  `stub`, `in production`, `placeholder`, `for M9`, `when … is wired`.

### GitHub issue state (after this session's sweep)

- Was 17 open / 170 closed. Now **57 open / 130 closed**, plus **15 new** (#201–#215) = 202 total.
- 95 reconciliation comments posted; 40 issues reopened; 4 relabelled (#193, #197, #199, #200).
- Full plan with every comment text: `ISSUE_ACTIONS.md` in the session scratchpad (see §5).

---

## 3. Owner decisions

| Question | Decision | Date |
| --- | --- | --- |
| Branch base | **Merge `main` into the working branch** — one branch, one PR to `alpha`. | 2026-09-02 |
| Closed-but-stubbed issues (M7/M9/M10) | **Reopen them.** Milestone burndown regression accepted; accuracy wins. | 2026-09-02 |
| OpenAPI / Swagger UI | **Defer** until the axum router exists. Record as blocked in proposals. | 2026-09-02 |
| Merge vs port (after I corrected my under-statement of the work) | **Do the full port now** — keep every deliverable rather than deferring the MusicBrainz hardening. | 2026-09-03 |

Standing instructions:

- Deep analysis / deep planning → **sequential Fable 5** agents (fall back to Opus per call, retry
  Fable next call). Implementation → Sonnet/Haiku; Opus when genuinely complex.
- **No PR stacking.** Everything lands on this one branch.
- Commit **and push** after each unit of work; update the relevant GitHub issue each time.
- Keep this file current as work proceeds.

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

The deep audit wrote these to the **session scratchpad**, which is session-scoped and **will not
survive into a new session**:

```text
/private/tmp/claude-501/-Users-lance-manasse-…/07a21012-…/scratchpad/
    BRIEF.md                 audit ground-truth brief
    audit_<domain>.md        9 domain audit reports (mm-core, mm-providers, mm-cli, mm-cloud,
                             mm-export, mm-server, ui-ffi, ci-release, docs)
    PLAN_proposals.md        28 ranked proposals with evidence
    ISSUE_ACTIONS.md         95-row action table + every comment text (137 KB)
    comments/C-*.md          the 95 posted comments
    newissues/N-*.md         bodies for #201–#215
```

If they are gone, the audit workflow script is saved and re-runnable:
`~/.claude/projects/…/07a21012-…/workflows/scripts/mm-state-audit-wf_0fb16a09-0df.js`
(run ID `wf_0fb16a09-0df`; read `journal.jsonl` in its transcript dir before re-running).

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

1. Read this file, then `.claude/CLAUDE.md` for the project rules.
2. `git log --oneline -5` and `git status` — confirm where the branch is.
3. Check for in-flight background agents (steps 7 and 8) — if their work is absent from the tree,
   re-run it from the descriptions above.
4. `export PATH="$HOME/.cargo/bin:$PATH"` before any cargo command (cargo is not on the default
   PATH in this environment).
5. Pick up the status board in §1 at the first ⏳ row.

---

## 8. Next actions

1. **Open the PR** from `claude/musicbrainz-api-migration-7jxszn` → `alpha`. Deliberately not
   created this session (no-PR-stacking rule) — it is the owner's call.
2. Expect CI to run for the first time on that PR. See the warning in §1.
3. Run the post-PR dev-cache cleanup from `.claude/CLAUDE.md` once the PR exists.
4. Start on the ranked proposals. The top five, in order: #201 (`scan --execute` data loss),
   #128 (Test Mode not enforced), #214 (cut a real pre-release), #204 (CI for `alpha`/`beta`),
   #205 (stubs must not report success).
5. Four reopened issues may be better recorded as *not planned* than as open work — the owner
   should decide: **#74** (provider auto-registration via `inventory`, never adopted),
   **#80** (the five stub music providers), **#104** and **#106** (App Store / Microsoft Store
   submissions).

## 9. Change log for this handoff file

- **2026-09-03 (later)** — provider rewiring and the full documentation rewrite landed; commit
  table, final verification numbers and next actions recorded. Added the warning that no CI has
  ever run on this branch.
- **2026-09-03** — merge + MusicBrainz port landed (`58ea003`); issue sweep complete (95 comments,
  40 reopens, #201–#215 filed); documentation scope recorded; pagination gap documented.
- **2026-09-02** — created. Recorded branch divergence, the 9-crate/1,392-test inventory, the
  M7/M9/M10 stub findings, the owner decisions, and the pending merge analysis.
