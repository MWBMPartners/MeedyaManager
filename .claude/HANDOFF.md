# MeedyaManager — Session Handoff

> **(C) 2025-2026 MWBM Partners Ltd**
>
> **Purpose:** the single place to look to resume work after any interruption.
> Records *verified* state only — never aspirational state. Update after every task.

**Last updated:** 2026-09-03
**Updated by:** Claude Opus 5 (1M context), session `07a21012`
**Working branch:** `claude/musicbrainz-api-migration-7jxszn` → will PR into **`alpha`**
**Branch HEAD:** `9f3719b` (end of Round 2)

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
| 11 | **Branch consolidation** — 4 stale branches folded into this one | ✅ done (see §9) |
| 12 | Delete the 4 consolidated branches | ✅ done — all 4 deleted, `archive/*` tags kept |
| 13 | **Round 1** — 7 alpha-readiness packages | ✅ done (see §10) |
| 13b | **Round 2** — release readiness | ✅ done (see §10c) |
| 13c | Round 3 — scope being planned | 🔄 in progress |
| 14 | **Open the PR to `alpha`** | ⏳ owner's call — not created, per the no-PR-stacking rule |
| 15 | Post-PR dev-cache cleanup (per `.claude/CLAUDE.md`) | ⏳ after the PR exists |

### Commits on this branch (all pushed)

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

**Latest verification (end of Round 2):**

```text
cargo fmt --all --check                              clean
cargo test --workspace                    1,304 passed, 0 failed
cargo clippy --workspace --all-targets -D warnings   0 issues
cargo deny check          advisories ok, bans ok, licenses ok, sources ok
actionlint                                           clean
version                                        1.4.0-alpha.1
```

`mm-cloud`'s 40 clippy errors are gone (#200), so the gate no longer needs `--exclude mm-cloud`;
and `--exclude mm-gtk` was dropped too, since the workspace `exclude` key makes it redundant.

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
  `mm-update` is the 9th crate. Root `Cargo.toml` now carries `exclude = ["crates/mm-gtk"]` so the
  crate resolves standalone (issue #199, fixed and closed 2026-09-03).
- **44,183 Rust LOC; 1,392 `#[test]`/`#[tokio::test]` functions.** Docs claiming "8 crates" and
  217/399/444 tests are stale.
- `cargo test --workspace`: **1,303 passed, 0 failed** (1,207 after the `main` merge → 1,240 after
  the provider rewiring → 1,244 after the #196 merge → 1,303 after Round 1).
- Version is **1.3.0** in `Cargo.toml`, `Info.plist` and `Package.appxmanifest`. Versions 1.3.1
  and 1.3.2 were **never cut** despite changelog entries for them. Linux/WinGet manifests are
  unsynced (snap/deb say `0.9.0`, WinGet `1.0.0`, flatpak has a placeholder commit pin).
- **No public release exists.** The only GitHub release is *"MetaMancer v1.0-M1"* (2025-06-16) —
  the pre-rename project name. Only tag is `v1.0-M1`. The archive tag `v1.5-M6-python-final`
  referenced by `.claude/CLAUDE.md` **does not exist** (this is why #19 was reopened).
- `LICENSE` is now tracked and byte-identical to the official GNU GPL-2.0 text, and is staged into
  every package (#207, fixed 2026-09-03). This also fixed an existing Flatpak build break.

### Completion reality

- **M10 server (#120–#127)** — `mm-server` never builds an axum router (no `.route(` call exists);
  `crates/mm-cli/src/commands/serve.rs:337-342` prints *"Server stub: exiting cleanly"*. The repo
  has **zero `.html` files**, so #124's web frontend has no deliverable.
- **M9 export (#112–#119)** — `sqlx`/`tiberius` declared; no pool is ever created, no SQL executed.
- **M7 cloud (#94–#102)** — no real network calls; OAuth flows exist only as comments.
- ~~Test Mode is never enforced~~ — **fixed** (#128, `6894105`). All three consumers now route
  through `integrity::*_safe`. Three bugs were found doing it, the worst being that `temp_path`
  produced an unparseable extension so the standard write path had **never worked on a real file**.
- ~~`meedya scan --execute` can destroy files~~ — **fixed** (#201, `cf7a6ec`), verified end-to-end
  with the built binary: both files survive, the second is flagged CONFLICT, exit 2. A **path
  traversal from tag data** was found in the same code and closed — `simulate_rename` joined
  unsanitised template directory components, so a tag value could escape the output directory
  (the regression test wrote to `/etc`). It was reachable through the API `mm-ffi` and `mm-gtk`
  already call, so it was never CLI-only.
- **Issues #165–#191 (27 DJ/tagging issues) are correctly closed** — deliberately moved to
  MeedyaSuite-core by owner direction on 2026-05-29. Do not treat as bogus closures.
- Only **2** `TODO`/`FIXME` markers repo-wide; unfinished work is described in prose. Grep for
  `stub`, `in production`, `placeholder`, `for M9`, `when … is wired`.

### GitHub issue state (after this session's sweep)

- Was 17 open / 170 closed. After the sweep: 57 open / 130 closed, plus 15 new (#201–#215).
  After Round 1 and the branch consolidation: **64 open**, 202 total.
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

## 11. Change log for this handoff file

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
