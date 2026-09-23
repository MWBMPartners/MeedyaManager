# CLI as a Scripting API

> **(C) 2025-2026 MWBM Partners Ltd**

`help/cli-reference.md` documents the `meedya` CLI for a human reading a terminal — every flag,
with examples. This page documents it as a **machine interface**: the exact exit codes a script
should branch on, the exact JSON shape (field names and types) each `--json`-capable command emits,
and which commands are currently stubs that a script should not rely on doing real work. Every
shape below is copied from the real `#[derive(Serialize)]` struct in
`crates/mm-cli/src/commands/*.rs`, not reconstructed from the human-readable output.

## Exit codes

Defined in `crates/mm-cli/src/output.rs::ExitCode`:

| Code | Constant | Meaning |
| ---- | -------- | ------- |
| `0` | `SUCCESS` | Completed successfully |
| `1` | `ERROR` | Failed |
| `2` | `PARTIAL` | Some items succeeded, some failed (see per-command notes below for exactly what triggers this) |
| `3` | `NOT_IMPLEMENTED` | Arguments parsed and validated, but the feature is not built yet — no work was attempted at all |

`NOT_IMPLEMENTED` is a deliberate, checkable signal that a script can distinguish from a real
failure. As of this commit, three commands unconditionally return it for their main operation:
`meedya lookup`, `meedya export` (unless `--show-schema` is passed), and `meedya serve` (unless
`--show-routes` or `--check-config` is passed).

**As of 2026-09-23 (issue #180), two more cases return it too, as a safety catch rather than
because the feature is unbuilt:**

- `meedya service install` now returns it on **every** platform. On Windows this is the
  pre-existing, permanent refusal (see the `meedya service` section below). On Linux and macOS
  it is new: real organising is switched off while known data-loss problems are fixed, so
  installing a service that would run it is refused too, and nothing is installed.
- `meedya watch --organize` **without the global `--dry-run` flag** now returns it as well,
  for the same reason, and moves nothing — even with `--yes`. `meedya --dry-run watch
  --organize` is unaffected, because a dry run moves nothing anyway.

Both refusals are controlled by one constant, `ORGANISING_SWITCHED_ON` (currently `false`) in
`crates/mm-cli/src/commands/watch.rs`. Everything below that describes `watch --organize` and
`service install` as fully working describes the built behaviour, which is what runs once that
constant is set back to `true` — not what happens today.

Every per-command exit code below describes a deliberate `Ok(ExitCode::…)` return in that
command's `run()` function. On top of these, an unexpected failure that a command does not
explicitly handle (permission denied, disk full, a malformed input file) propagates as an
`anyhow::Error` instead, which `main.rs` catches and turns into exit code 1 (`ERROR`) regardless
of which command was running — this path is not re-derived per command below.

## A gotcha that affects every JSON consumer of `meedya service`

`service install`, `service uninstall`, `service start` and `service stop` **ignore the global
`--json` flag entirely** — they always call `output::print_error`/`print_success` (coloured human
text), regardless of `ctx.output`. Only `service status` checks `ctx.output` and emits JSON. A
script that runs `meedya --json service start` and tries to parse stdout as JSON will get plain
text, not an error and not JSON — verified directly against `crates/mm-cli/src/commands/service_cmd.rs`,
where `install`, `uninstall`, `start` and `stop` never reference `OutputFormat` at all.

**One exception, since 2026-09-23 (#180):** while organising is switched off, the refusal from
`service install` on Linux and macOS *does* honour `--json`, printing
`{"status": "switched_off", "message": "…"}` with exit code `3`. Everything else in `install`
— including the Windows refusal and the `--dry-run` preview — is still plain text.

## A note on embedded Rust `Debug` strings

Several JSON payloads below embed a Rust `{:?}` Debug-formatted string as a JSON string value
rather than a fully structured sub-object — e.g. `debug`'s `classification.group` is literally
`format!("{:?}", classification.group)`, and `rule tags`' `kind` field is
`format!("{kind:?}")` (producing values like `Metadata("title")` or `Virtual(Duration)`). These
are valid JSON strings, but their *content* is Rust's derived Debug syntax, not a stable
language-agnostic format — a script that wants to branch on the enum variant should match on the
leading identifier rather than assuming a fixed schema for what follows it.

## `meedya scan`

Fully implemented (delegates to `mm_core::renamer::simulate_rename`/`execute_rename`).

**Exit code:** `SUCCESS` normally; `PARTIAL` if any rename conflict remains unresolved after the
configured conflict strategy runs, or if any individual rename errors during `--execute`;
`ERROR` if the path is not a directory, or if `--execute` without `--yes` is refused at an
interactive confirmation prompt.

**JSON shape** (`ScanOutput`):

```json
{
  "directory": "string",
  "total_files": 0,
  "classification_summary": [{ "group": "string", "count": 0 }],
  "rename_previews": [
    { "source": "string", "destination": "string", "conflict": false, "unchanged": false }
  ],
  "summary": {
    "total": 0, "renamed": 0, "unchanged": 0, "conflicts": 0, "executed": false
  }
}
```

## `meedya debug`

Fully implemented. **Exit code:** `SUCCESS` always, once the file is confirmed to exist
(`ERROR` if it does not).

**JSON shape** (`DebugOutput`):

```json
{
  "file": "string",
  "classification": { "group": "string", "format": "string", "class": "string", "quality": "string" },
  "tags": { "title": ["string", "..."] },
  "audio_properties": {
    "duration_secs": 0.0, "bitrate_kbps": 0, "sample_rate_hz": 0,
    "channels": 0, "bits_per_sample": 0
  },
  "cover_art": { "size_bytes": 0, "mime": "string" },
  "companions": [{ "path": "string", "companion_type": "string" }]
}
```

`classification.*`, `tags` values are Debug-formatted per the note above.
`audio_properties`/`cover_art` are `null` when the file has none.

## `meedya edit`

Fully implemented, two-phase (validate-then-apply, so a batch is atomic — see the module doc in
`edit.rs` for why, referencing issue #206). **Exit code:** `ERROR` if the file does not exist or no
edit flag was given at all; `SUCCESS` if every requested action is valid and (outside `--dry-run`)
applies cleanly; `PARTIAL` in either of two distinct cases that a script should tell apart by
reading `actions[].success`, not just the exit code:

- **Phase-1 (validation) failure** — at least one requested operation is invalid (e.g. an
  unmapped tag key). In this case **none** of the batch is applied, not even the operations that
  were themselves valid — this happens before any file I/O, regardless of `--dry-run`.
- **Phase-2 (apply) failure** — every operation validated, but a real I/O error occurred while
  applying one of them. Operations run independently in this phase, so earlier and later actions
  in the same batch can still have succeeded even though one failed.

**JSON shape** (`EditOutput`):

```json
{
  "file": "string",
  "actions": [
    { "action": "string", "key": "string", "value": "string", "success": true, "error": null }
  ],
  "dry_run": false,
  "written_to": "string"
}
```

`written_to` is present **only** when Test Mode diverted the write to a `_MeedyaManager` copy
(`#[serde(skip_serializing_if = "Option::is_none")]`) — outside Test Mode the field is omitted
from the JSON entirely, not emitted as `null`.

## `meedya rule`

Four subcommands, all fully implemented.

### `rule validate <template>`

**Exit code:** `SUCCESS` if the template parses, `ERROR` otherwise.

```json
{ "template": "string", "valid": true, "error": null, "ast": "string" }
```

`ast` is the parsed AST rendered with `{ast:?}` (Debug format) — a debugging aid, not a stable
schema. `null` on failure; `error` is `null` on success.

### `rule tags`

**Exit code:** always `SUCCESS`. Output is a bare JSON array (not wrapped in an object):

```json
[{ "name": "string", "kind": "string" }]
```

`kind` is one of `Metadata("<key>")`, `Virtual(<VariantName>)`, `Custom("<key>")` — Debug-formatted.

### `rule test <template> <file>`

**Exit code:** `ERROR` if the file does not exist or evaluation fails; `SUCCESS` otherwise.

```json
{ "template": "string", "file": "string", "result": "string", "error": null }
```

### `rule legacy <template>`

**Exit code:** always `SUCCESS`.

```json
{ "template": "string", "legacy_keys": ["string"] }
```

## `meedya watch`

> **⚠️ `--organize` without `--dry-run` is switched off as of 2026-09-23 (issue #180) — see the
> exit-codes section above.** It refuses immediately, before the watcher starts, prints an
> error (or, with `--json`, `{"status": "switched_off", "message": "…"}`), and returns
> `NOT_IMPLEMENTED` (`3`). The description below is the built behaviour,
> which applies once `ORGANISING_SWITCHED_ON` is set back to `true`; a `--dry-run` run is
> unaffected and always works as described.

Fully built, including `--organize` (issue #180) — a foreground watcher that, with
`--organize`, also renames/moves files by delegating each settled folder to `scan::run`
internally (same engine as `meedya scan --execute`, so it inherits every safety rule that
command has: the write lock, the disc-image whole-folder protection, Test Mode hygiene).

**New flags added by #180:**

| Flag | Default | Meaning |
| ---- | ------- | ------- |
| `--yes` | off | Skip the one-off confirmation prompt before `--organize` starts moving files. Required in practice for any non-interactive use (a script, a cron job, the background service) — without it, an attended terminal is asked to confirm once at start-up, and a non-interactive stdin is treated as pre-confirmed already. |
| `--settle-secs <N>` | `2` | How many seconds a file must go untouched before `--organize` will act on it. Exists so a file still being copied in is never organised half-written. |

**Exit code:** as of 2026-09-23, `--organize` without the global `--dry-run` flag returns
`NOT_IMPLEMENTED` (`3`) immediately — the safety catch above runs before any of the following
is reached. With `--dry-run`, or without `--organize` at all: `ERROR` if no folders resolve
(neither args nor config) or a given path is not a directory — this now applies to `--organize`
too (previously it returned `NOT_IMPLEMENTED` before folders were even looked at); `ERROR` also
if an attended terminal declines the `--organize` confirmation prompt. Otherwise the process
runs until Ctrl+C, then returns `SUCCESS`. There is no exit code that reflects organising
failures encountered *during* the run
— those are logged to stdout/stderr as they happen (a per-folder failure is reported and that
folder's files are dropped; a folder blocked by another process holding the write lock is
retried after another settle window; see `help/background-service.md`), but the watcher keeps
running regardless and always exits `SUCCESS` when stopped with Ctrl+C.

**Conflict handling while organising is always forced to `"skip"`**, regardless of
`config.rename.conflict_strategy` — a warning is printed once, at start-up, if the configured
strategy was something other than `"skip"`. This avoids a known runaway-renaming defect, tracked
as issue #224, that the `"rename"` strategy can hit when the watcher re-evaluates the same
folder on every settle.

**Startup JSON** (`WatchStartOutput`, printed once):

```json
{ "folders": ["string"], "recursive": true, "organize": false, "settle_secs": 2 }
```

**Per-event JSON** (`WatchEventOutput`, printed as one JSON object per line as events arrive — this
is JSON Lines, not a single JSON document):

```json
{ "timestamp": "HH:MM:SS", "event_type": "Created|Modified|Deleted|Renamed", "path": "string" }
```

## `meedya lookup` — stub

**Always returns `NOT_IMPLEMENTED`.** No provider is queried; `mm-providers` (19 real/stub
providers, fully implemented — see `docs/api/rust-crates.md`) is never called from this command.
The query and `--provider`/`--auto`/`--apply`/`--batch` flags are parsed but deliberately not
echoed back, precisely so the output does not imply a search happened.

```json
{ "status": "not_implemented", "message": "string", "planned_providers": ["string"] }
```

## `meedya config`

Five subcommands, all fully implemented (Test Mode enable/disable is real; profile export/import
uses `mm_core::settings_bundle`). **Exit code:** `SUCCESS` for every successful path; each
subcommand also has one explicit `ExitCode::ERROR` return for a specific expected-failure case —
`config show` (JSON serialisation failure of the config — effectively unreachable in practice),
`config init` (target file already exists), `config test-mode <bad-action>` (unrecognised action
name).

### `config show`

Serialises `ctx.config` (the full `mm_core::config::AppConfig`) directly — there is no
CLI-specific wrapper struct. Its shape matches `settings.json5` field-for-field; see
`help/configuration.md` for the field reference (not duplicated here to avoid drift between two
owners of the same schema).

### `config path`

```json
{ "config_path": "string", "exists": true }
```

### `config init [path]`

```json
{ "path": "string", "created": true }
```

### `config export <output>` / `config import <profile>`

`export` uses a dedicated struct; `import` uses an ad hoc `serde_json::json!` value with a
different shape — these are **not** symmetric:

```json
// export
{ "path": "string", "action": "export", "success": true }
```

```json
// import
{ "action": "import", "success": true, "bundle_version": "string", "files_written": ["string"] }
```

### `config test-mode <on|off|status|commit|revert>`

`on`/`off`/`status` share one shape (`TestModeOutput`):

```json
{ "enabled": true, "tracked_files": 0, "action": "on|off|status" }
```

`commit`/`revert` each use their own ad hoc `serde_json::json!` shape instead:

```json
// commit
{ "action": "commit", "files_committed": 0, "success": true }
```

```json
// revert
{ "action": "revert", "files_kept": 0, "success": true }
```

## `meedya report-bug`

Fully implemented. **Exit code:** always `SUCCESS`.

```json
{
  "system": { "os": "string", "arch": "string", "meedya_version": "string" },
  "health": [{ "name": "string", "status": "string", "message": "string" }],
  "config_path": "string",
  "watch_folders": ["string"],
  "log_tail": ["string"]
}
```

`log_tail` is `null` unless `--include-logs` is passed and a log file was found (up to the last
200 lines). In human mode this same data renders as Markdown instead of JSON — the `--json` flag
changes the format, not the content.

## `meedya export` — schema generation is real, exporting is not

**`--show-schema` is fully functional** — it renders genuine dialect-specific DDL from
`mm_export::SchemaBuilder` and returns `SUCCESS`. Every other invocation (with or without
`--dry-run`) **always returns `NOT_IMPLEMENTED`**: no directory is scanned and no database
connection is opened, regardless of the DSN given, because — as `docs/api/rust-crates.md`
documents in detail — `mm-export`'s backend structs never make a real database call at all, not
even when driven by a real caller. **Exit code:** `ERROR` only if `--db` is empty; `SUCCESS` for
`--show-schema`; `NOT_IMPLEMENTED` otherwise.

```json
{ "status": "not_implemented", "backend": "SQLite|MySQL|MariaDB|PostgreSQL|SQL Server", "connection": "string", "message": "string" }
```

`connection` is the DSN with credentials redacted (scheme + host only) — never the raw DSN.

## `meedya serve` — routes and config-check are real, serving is not

**`--show-routes` and `--check-config` are fully functional.** Every other invocation **always
returns `NOT_IMPLEMENTED`**: as `docs/api/rust-crates.md` documents, no axum router is ever built
anywhere in this codebase, so nothing actually listens on the reported address. **Exit code:**
`ERROR` if `--check-config` finds a validation problem (missing JWT secret, missing TLS paths
without `--no-tls`) or if the JWT service itself fails to initialise; `SUCCESS` for
`--show-routes` and a passing `--check-config`; `NOT_IMPLEMENTED` otherwise.

```json
{
  "started": false,
  "address": "host:port",
  "tls_enabled": true,
  "cors_origins": ["string"],
  "media_root": "string",
  "message": "string"
}
```

`started` is always `false` in this release — there is no code path that sets it to `true`.

## `meedya service`

> **⚠️ `install` is switched off on every platform as of 2026-09-23 (issue #180).** On Linux and
> macOS this is new: it now returns `NOT_IMPLEMENTED` (`3`) and installs nothing, for the same
> reason `watch --organize` is switched off — the service it would install runs
> `meedya watch --organize --yes`, which is exactly the switched-off, real-organising path. The
> Windows refusal below is separate and unrelated, and was already in place. `--dry-run`
> previews still work on Linux/macOS, unaffected by this. `uninstall`, `start`, `stop` and
> `status` are unaffected on any platform.

`install` is fully built for **Linux and macOS** (issue #180) — it writes and enables/loads
a real systemd user unit or launchd LaunchAgent, running `meedya watch --organize --yes`. On
**Windows it always returns `NOT_IMPLEMENTED`** and refuses outright, regardless of `--dry-run`:
a program registered via `sc create` has to answer the Windows Service Control Manager within
about thirty seconds of starting (`meedya` does not speak that protocol) and would run as the
LocalSystem account, reading a different settings file than the one the installer edited.
`meedya service install` prints this explanation and points at Task Scheduler as the supported
alternative instead of registering something that would not work.

**Exit code:** `install` — as of 2026-09-23, `NOT_IMPLEMENTED` unconditionally on **every**
platform without `--dry-run` (Windows for its own pre-existing reason, Linux/macOS for the new
safety catch above); with `--dry-run` on Linux/macOS, `SUCCESS`, printing what would be
registered and writing nothing — Windows still returns `NOT_IMPLEMENTED` regardless of
`--dry-run`. Once the safety catch is lifted, a real (non-dry-run) Linux/macOS install returns:
`SUCCESS` on a successful install; `ERROR` if the current executable's path cannot be resolved
and no `--bin-path` was given, or if the underlying OS command fails. `uninstall`/`start`/`stop` return
`SUCCESS` or `ERROR` from the underlying `mm_core::service` call — on Windows these still shell
out to `sc.exe`, but since `install` never registers anything there, they have nothing to act
on. `status` returns `SUCCESS` only when the service is actually `Running` — `Stopped`,
`NotInstalled` and `Unknown` all return `ERROR`, which is unusual among `meedya` subcommands
(most treat "nothing to report" as success) and worth a script author noting explicitly.

`status --json` (the only subcommand that honours `--json` — see the gotcha above) emits an ad hoc
shape, not a `#[derive(Serialize)]` struct:

```json
{ "service": "meedyamanager", "status": "running|stopped|not-installed|unknown", "running": true }
```

`status` is the lowercase `Display` form of `ServiceStatus` (`crates/mm-core/src/service.rs`),
not the enum's variant name — note the hyphen in `not-installed` specifically.
