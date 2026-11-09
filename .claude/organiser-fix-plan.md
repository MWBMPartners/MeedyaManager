<!-- (C) 2025-2026 MWBM Partners Ltd -->
<!-- Saved from the planning agent's output on 2026-09-25 so it survives a wiped scratchpad.
     Paths were shortened: <repository> is the repo root, <session scratchpad> the session's temp folder. -->

# Organiser fix plan — stages (c) to (g) for #180

**Written:** 2026-09-25, by the planning agent (Opus), for the orchestrator of session `07a21012`.
**Changed nothing** in any repository or on GitHub. The only file written is this one.

**Built on:** the local branch `review/round2` at `083ad0f`, plus the gap-fix commit a Sonnet builder
is making right now in `.claude/worktrees/round2`. That commit was not finished when this was written.
I read its work in progress (read-only) so that this plan uses the same names:

- `DiscFolder.missing_files`: a non-empty list means the folder is "held back";
- `held_back` and `missing_files` in the `scan --json` disc folder entries;
- `describe_held_back()` in `scan.rs`;
- `has_aria2_control_file()` in the watcher;
- `partition_for_scan(files, scan_root)` now examines every folder between a file and `scan_root`.

**Line numbers are at `083ad0f`.** The gap-fix commit will move lines in `scan.rs`, `disc/mod.rs` and
`watcher/mod.rs`. Where they have moved, search for the function named.

---

## 1. The plan on one page

| Stage | What it does, in plain words | Main files | Can run beside |
| --- | --- | --- | --- |
| **c1** | Stops the organiser renaming the same file again and again | `renamer/mod.rs`, `scan.rs`, `watch.rs` | d3, f1 |
| **c2** | Keeps a rip's files together on the organiser's own path; holds back folders with something still arriving; the organiser reports held-back folders | `scan.rs`, new `mm-core/src/hold.rs`, `watch.rs` | d3, f1 |
| **d1** | Leaves existing files alone unless asked (`--organize-existing`); an honest question before starting; refuses to run unattended without `--yes` | `watch.rs` | d3, f1 |
| **d2** | Ctrl+C really stops it (press twice to stop at once); a copy that is cut short never leaves a half file under a real name | `watch.rs`, `scan.rs`, `renamer/mod.rs` | d3, f1 |
| **d3** | The background service: install is honest and starts it; definitions are quoted and escaped; logs leave `/tmp`; status tells the truth | `mm-core/src/service.rs`, `service_cmd.rs` | c1, c2, d1, d2, e1, f1 |
| **e1** | "Settled" means really finished; a folder moved in whole is organised; files in hidden folders (such as a drive's bin) are left alone | `watch.rs`, `mm-core/src/watcher/mod.rs` | d3, f1 |
| **e2** | Where files go, and when not to move at all: the configured output folder, real paths, checks before starting, `~` in settings, `dry_run` in settings, Test Mode | `watch.rs`, `context.rs`, `mm-core/src/config/mod.rs`, `service_cmd.rs` | f1 |
| **e3** | Reporting and staying power: a status file and "Needs your attention" in `meedya service status`; one organiser at a time; waiting politely when another copy is busy | `watch.rs`, new `organiser_status.rs`, `service_cmd.rs`, new schema | f1 |
| **f1** | Lock documentation: stop telling people to delete `meedya.lock` (closes #49) | help pages + one documentation-check test | everything |
| **f2** | Organiser and service documentation, rewritten to match the code | help pages, `docs/`, notes | — |
| **g** | Switch organising back on | `watch.rs`, `service_cmd.rs`, docs | — |

**Order:** c1 → c2 → d1 → d2 → e1 → e2 → e3 → f2 → Codex review of the whole organiser → g.
d3 and f1 run beside that line in their own worktrees. d3 must land before e2. (Details in §6.)

**Owner decisions:** five, in §7. None of them blocks c1 to f2. Decision 1 (the final yes) blocks g.

---

## 2. What I checked, and how

- **Read the code at `review/round2` (`083ad0f`):**
  - `crates/mm-cli/src/commands/watch.rs`, `scan.rs`, `service_cmd.rs`, `context.rs`, `output.rs`;
  - `crates/mm-core/src/renamer/mod.rs`, `disc/mod.rs`, `watcher/mod.rs`, `service.rs`, `config/mod.rs`,
    `test_mode.rs`, `state/mod.rs` (the lock messages), and the rule engine's location-based fields;
  - the FFI watcher (`mm-ffi/src/uniffi_api.rs:556`), which only forwards events and never organises.
- **Read** `.claude/HANDOFF.md` §0, §0b and §15; Codex's partial notes; issues #180, #231, #224, #225,
  #226, #235 and #49 (read-only `gh` calls).
- **Checked MeedyaSuite-core** at the pinned `222ca75` and at the local head `aad49c7`.
  - It has **no** file watcher, settling logic, service installer, write lock or organiser. So nothing
    in this plan can be taken from upstream.
  - It does have its own cue sheet parser (`meedya-library-import/src/cuesheet.rs`, switched on by the
    `full` feature). This plan builds no cue parsing, so that is only noted in §8 as a suggestion.
- **Four live checks on this Mac.** Each one was harmless: it registered, installed and moved nothing.
  1. `launchctl list <unknown label>`: standard output is empty, and "Could not find service" goes to
     the error stream (exit 113). **This proves `meedya service status` reports "stopped" when nothing
     is installed on macOS** (O13 below).
  2. `launchctl load <missing file>`: prints "Load failed: 5" but **exits 0**. So `service install`
     would report success after a failed install (O12).
  3. `launchctl bootstrap gui/501 <missing file>` exits **5**, and `launchctl print gui/501/<unknown>`
     exits **113**. The newer commands do report failure, so stage d3 can rely on them.
  4. A plain `meedya watch` (this morning's build; plain watch never moves anything), with an empty
     settings folder, pointed at a scratch folder through the `/tmp` symlink.
     - Events arrived as `/private/tmp/...`, not `/tmp/...`.
     - **This proves finding #15:** the organiser matches events to watch folders by the path as
       typed, so it would ignore every event (O21).
     - The same output showed log lines carrying colour codes when written to a file (a small point,
       fixed in e3).
     - The scratch folder was deleted afterwards.
- **Confirmed from tokio's own documentation** (`tokio-1.50.0/src/signal/ctrl_c.rs`): once a program
  listens for Ctrl+C, every later Ctrl+C is captured "for the duration of the entire process". So a
  second Ctrl+C during the start-up sweep does nothing at all (O9).

**What I did not verify** is listed in §9.

---

## 3. Every organiser defect still open

**How to read this.** O-numbers are new labels for this plan. "§0b #n" is the 15 September review's
numbering. "Holds?" is my re-check against the code at `083ad0f`, with the gap-fix commit assumed to have
landed.

### 3a. Summary table

| # | Defect, in plain words | Where (at `083ad0f`) | Holds? | Stage |
| --- | --- | --- | --- | --- |
| O1 | Renames the same file over and over (`01 - 01 - 01 - song.wav`) | `watch.rs:244-265`; no check in `renamer/mod.rs:486-572` | Yes (§0b #2) | c1 |
| O2 | A rip's artwork or bonus tracks in a **subfolder** are moved out of the rip when the organiser handles an event in that subfolder | `watch.rs:250-256` passes the event folder as the scan folder; `scan.rs:627` | Yes, even after the gap fix (§0b #1, remaining half) | c2 |
| O3 | A rip's log and artwork are moved away **before its `.cue` arrives**: a `.bin` still downloading, or a finished music-CD `.bin` with no cue yet | `disc/mod.rs` Rule 4 (lone `.bin` ignored; doc near `:458`, test `:1381`); no folder-level download check in `scan.rs:598-627` | Yes (new) | c2 (e1 adds a second guard) |
| O4 | Held-back folders are not reported by the organiser; in the background nobody reads the screen | `watch.rs:333-349` ignores everything but the exit code | Yes (new; required before g) | c2 (log), e3 (status) |
| O5 | Starting it reorganises the **whole existing library** | `watch.rs:291-309`, called always at `:451-455` | Yes (§0b #3) | d1 |
| O6 | The start question mentions only files "as they arrive" | `watch.rs:566-569` | Yes (§0b #3) | d1 |
| O7 | With no terminal and no `--yes`, it treats silence as "yes". Services installed by older builds run without `--yes` and would start moving files the moment organising is switched on | `watch.rs:564-565` using `scan.rs:315-317` | Yes (#180 comment) | d1 |
| O8 | Nested watch folders disagree: the sweep files things under the outer folder, events under the inner one, so each restart moves files again | `watch.rs:301-302` against `:222-228` | Yes (§0b #13, second half) | d1 |
| O9 | Ctrl+C cannot stop the sweep; a second Ctrl+C is swallowed; "Watcher stopped" is printed while files are still moving | `watch.rs:639-649`; tokio docs | Yes (§0b #3) | d2 |
| O10 | Copy mode writes straight to the final name, so a stop, crash or power cut mid-copy leaves a **half file under a proper name** in the library | `renamer/mod.rs:665-674` | Yes (new) | d2 |
| O11 | macOS `service install` starts the service straight away while saying it will start at login | `service.rs:346-347`, `:394`; message at `service_cmd.rs:173-177` | Yes (§0b #4) | d3 |
| O12 | macOS install reports success even when launchd failed to load it | `service.rs:394` (`launchctl load` exits 0 on failure; checked live) | Yes (new) | d3 |
| O13 | `service status` says "stopped" when nothing is installed (macOS proven; Linux by reading) | `service.rs:182-188` (standard output only), `:422-447`, `:288-305` | Yes (new) | d3 |
| O14 | macOS service logs go to fixed names in `/tmp`, which another account on a shared Mac can pre-create or redirect | `service.rs:364-368` | Yes (§0b #14) | d3 |
| O15 | Unquoted program path in the Linux unit; unescaped text in the macOS file; restarts every 5 seconds for ever after a refusal | `service.rs:218`, `:337`, `:219-220` | Yes (§0b #17) | d3 |
| O16 | `service install --dry-run` prints one sentence, not the definition. Install never checks that any watch folder is set | `service_cmd.rs:134-147` | Yes (new) | d3 |
| O17 | "Settled" means only "no events for 2 seconds", so a slow or stalled copy is organised half-written, from half-read tags | `watch.rs:83-84`, `:176-191` | Yes (§0b #11) | e1 |
| O18 | A folder moved in whole is never organised. With an include list set, the folder's own event is thrown away as well | `watch.rs:149-163`, `:202-213`, `:333`; `watcher/mod.rs:346-353`, `:500` | Yes (§15 finding) | e1 |
| O19 | A file arriving inside a hidden folder is organised. The sweep skips hidden folders, but events do not. Deleting a file on a watched drive puts it in `.Trashes`, and the organiser would move it back out | `watcher/mod.rs:271-365` checks names only; the sweep skips at `:583-586` | Yes (new) | e1 |
| O20 | Ignores the configured output folder | `watch.rs:256` beats `rename.output_dir` through `scan.rs:363-368` | Yes (§0b #13) | e2 |
| O21 | A watch folder given through a symlink or a relative path: its events are ignored on macOS | `watch.rs:500-519`, `:222-228`, `:325-329` | **Yes, proven live** (§0b #15) | e2 |
| O22 | A settings file that exists but cannot be read is replaced by defaults with only a log line, so an unattended organiser uses the default template | `context.rs:71-78` | Yes (new) | e2 |
| O23 | `dry_run: true` in `settings.json5` is ignored by every command | `context.rs:88-93`; the setting is never read | Yes (new) | e2 |
| O24 | A `~` in settings paths is taken literally. The shipped example uses `~/Downloads/Media` and `output_dir: "~/Media/Organised"`. `scan --execute` can create a folder literally named `~` in the current folder | `config/settings.json5:64-66`, `:101`; no expansion anywhere in `crates/` | Yes (new; affects `scan` too) | e2 |
| O25 | Moves real files while Test Mode is on; the desktop apps refuse | `scan.rs:603-611` only skips tracked originals | Yes (new; owner decision 2) | e2 |
| O26 | An output folder on an unplugged drive would be **created on the start-up disk** (`/Volumes/Media` on a Mac). An output folder on another drive fails for every file, one error at a time | `renamer/mod.rs:658-663` (`create_dir_all`), `:677` (`rename` cannot cross drives) | Yes (new) | e2 |
| O27 | While another copy holds the write lock, it retries every 2 seconds for ever and prints the refusal each time; the sweep never retries | `watch.rs:333-356`, `:301-308` | Yes (§0b #12) | c1 (exact detection), e3 (waiting) |
| O28 | Conflicts and failed moves leave the queue with no record for later | `watch.rs:338` | Yes (§0b note) | e3 |
| O29 | Two organisers at once (a terminal run and the service) duplicate the work. Nothing stops the second | none | Yes (new) | e3 |
| O30 | The `watch.recursive` setting is ignored by `watch` | `watch.rs:580`, `:590` | Yes (new, minor) | e2 |
| O31 | Help pages tell people to delete `meedya.lock`, and describe the old process-number lock | `help/troubleshooting.md:238-249`, `help/cli-reference.md:194-224` | Yes | f1 |
| O32 | Help pages promise `/tmp` logs, "set to start at login", files "as they arrive", a 2-second settle, and a macOS "system-wide LaunchDaemon with `sudo`" that no code creates | `help/background-service.md:392-393`, `:494`; `help/faq.md:182`; and others found by the searches in f2 | Yes | f2 |
| O33 | The organiser re-files files that a person renamed by hand inside a watched folder | by design of any organiser | Yes | documented in f2 (owner decision 4) |

### 3b. Each defect in more detail

**O1 — the rename loop.**
- Take the template `<Track> - <Filename>`. `<Filename>` is the file's own name without its ending
  (`rule_engine/evaluator.rs:166-172`).
- The organiser renames `song.wav` to `01 - song.wav`. That rename is a new event in a watched folder.
  About 2 seconds later the folder is scanned again, and it becomes `01 - 01 - song.wav`, and so on for
  ever.
- Forcing "skip" does not help: each new name is free.
- Any template that uses the file's name, folder or full path (the location-based fields) can do this.

**O2 — rip subfolders on the organiser's path.**
- Layout: `Rip/Album.cue`, `Rip/Album.bin` and `Rip/Scans/cover.jpg`. A change lands in `Rip/Scans/`.
- The organiser calls `scan` on `Rip/Scans` alone (`organise_directory`, `watch.rs:250-256`).
- `scan` then calls `partition_for_scan(files, &args.path)` (`scan.rs:627`) with `Rip/Scans` as the
  boundary.
- The gap fix walks upward only as far as that boundary, so it never looks at `Rip/`. `cover.jpg` is
  organised out of the rip.
- The opt-in sweep does not have this problem, because its boundary is the watched folder.

**O3 — before the cue arrives.**
- *Layout 1:* `Rip/Album.bin.part` (still downloading) and `Rip/Album.log`, with the `.cue` further down
  the download queue. No image set exists yet, so `Album.log` is loose and is moved away.
- *Layout 2:* a finished `Album.bin` from a music CD. It has no sync pattern (§15 finding; a music
  CD's sectors never carry one), so without its `.cue` it forms no set. Its log and cover are moved
  away.
- In both cases, when the `.cue` arrives the folder is sealed, but the companions are already gone.

**O4 — reporting.**
- The gap fix makes `scan` print a "Held back" explanation. The organiser throws away everything except
  the exit code.
- In service mode the output goes to a log nobody reads, repeated every time the folder is scanned.

**O5 to O8 — the sweep.**
- `sweep_roots` runs `scan --execute --yes`, recursively, over every watched folder at every start
  (`watch.rs:291-309`). The only question asked is about files "as they arrive" (`:566-569`).
- Under a service there is no terminal, and `execute_pre_confirmed(false, false)` is `true`
  (`scan.rs:315-317`). So a service from an older build without `--yes` confirms itself.
- The outer folder's recursive sweep covers the inner watched folder too, so files the events filed
  under `/m/Incoming/...` are moved to `/m/...` at the next restart.

**O9 — Ctrl+C.**
- The first Ctrl+C completes `tokio::signal::ctrl_c()` (`watch.rs:639`). "Watcher stopped" is printed at
  once (`:641-644`).
- Then `event_handle.await` (`:649`) waits for the sweep, which never checks for a stop.
- From that moment tokio captures every further Ctrl+C, so nothing on the keyboard can stop it. Only a
  kill from another terminal, or closing the window, will.

**O10 — copies.**
- `std::fs::copy(source, destination)` writes the destination directly (`renamer/mod.rs:668`).
- A kill, a crash, a service stop at shutdown or a power cut mid-copy leaves a truncated file under the
  final, proper-looking name.
- The original is safe, but the library now holds a broken file. The organiser will then report it as a
  conflict for ever.

**O11 to O16 — the service.**
- On macOS, `RunAtLoad` together with `launchctl load` starts the service at once, but the message says
  "set to start at login … to start it now: meedya service start".
- `launchctl load` returns 0 even on failure (checked live). So a plist broken by an `&` in the program
  path "installs" successfully and never runs.
- `cmd_output` reads standard output only, so "Could not find service" is never seen and status says
  STOPPED.
- `/tmp/meedyamanager.stdout.log` is a fixed name in a folder shared by every account.
- `ExecStart={bin} …` breaks on a path with a space, and systemd treats `%` specially.
- `Restart=on-failure` with `RestartSec=5s` restarts a refusal (exit code 1 or 3) every 5 seconds. The
  start limit is never reached, because five starts take longer than the default 10-second window.
- `--dry-run` shows a sentence, not the file. Install does not check for watch folders, so the
  installed service fails at once.

**O17 — settling.**
- `take_settled` (`watch.rs:176-191`) is a per-file timer: 2 seconds with no event.
- A copy over a network share or USB that pauses for longer than that is organised mid-copy.
- On macOS and Linux the writer keeps writing to the moved file, so the bytes survive. But the tags
  were read from half a file, so the file goes to the wrong place ("Unknown Artist"). A download tool
  that resumes by path loses track of its file.
- The per-file timer also lets a folder be organised while its other files are still copying.

**O18 — folders moved in whole.**
- Moving a folder into a watched folder raises one event for the folder (macOS) or for the folder only
  (Linux, where the files already exist). That path is queued as if it were a file.
- It is grouped under its parent, and the parent is scanned non-recursively (`watch.rs:333`). The
  folder's contents are never looked at.
- If `watch.include_extensions` is set, `should_ignore` drops the folder's event outright: a folder has
  no ending, so it fails the include list (`watcher/mod.rs:346-353`).

**O19 — hidden folders.**
- `should_ignore` looks only at the file's own name.
- `watched/.Trashes/501/song.mp3` (a file deleted on an external drive whose root is watched), or
  Syncthing's `.stversions/`, passes the check, and the organiser moves it back into the library.

**O20, O26 — output folder.**
- The organiser always passes the watched folder as the output folder, which overrides the setting.
- Once the setting is honoured, two traps open:
  - a missing output folder is created by `create_dir_all`. On a Mac, `/Volumes/Media/Library` with the
    drive unplugged becomes a folder on the start-up disk;
  - an output folder on another drive fails for every file, because `rename` cannot cross drives.

**O21 — symlinked or relative folders.**
- Proven live: watching `/tmp/x` produces events under `/private/tmp/x`.
- `watched_root_for` uses `starts_with` on the typed path, so it returns `None`, and the event is quietly
  skipped (`watch.rs:325-329`).
- The same happens for a symlinked `~/Music` pointing at another drive, and for any relative path.

**O22 to O25 — settings and Test Mode.**
- A typo in `settings.json5` makes `CliContext::build` fall back to defaults with only a log line.
  Given folders on the command line (the Windows Task Scheduler route), the organiser then files
  everything by the default template.
- `dry_run: true` in settings is never read; only the command-line flag counts.
- `~` is never expanded.
- In Test Mode the organiser moves real files, while the apps refuse to rename at all.

**O27 to O29 — resilience.**
- Retrying is decided by guessing: "exit code 1 probably means the lock was busy". Code 1 also means
  "cannot take the lock at all" (for example, a permissions problem), which is then retried every 2
  seconds for ever.
- Conflicts and failed moves (exit code 2) are dropped with nothing recorded for later.
- Nothing stops a second organiser.

**O30.** The setting says "recursive"; `watch` reads only `--no-recursive`.

### 3c. The 15 September review's 18 findings, re-checked

| §0b # | Finding | Still holds at `083ad0f`? |
| --- | --- | --- |
| 1 | Cue moved away from an arriving bin | **Partly.** See §4. The cue itself and its own folder are now safe; subfolders on the organiser's path (O2), and files that arrive before the cue (O3), are not |
| 2 | Rename loop | **Yes** (O1) |
| 3 | Whole-library sweep, prompt, Ctrl+C | **Yes** (O5, O6, O9) |
| 4 | macOS install starts at once while saying otherwise | **Yes** (O11); blocked by the safety catch until g |
| 5 | Windows Test Mode flag never reaches the engine | **The harm is gone.** The Windows app refuses saves and Execute in Test Mode (`MetadataPage.xaml.cs:173-196`, `ScanPage.xaml.cs:191`). The flag is still in-app only (`MmCore.cs:315-336`). Not an organiser matter |
| 6 | Two processes can both hold the lock | **Fixed** (`599d921`, operating-system lock; Codex's partial notes agree) |
| 7 | Reused process number blocks for ever | **Fixed** (`599d921`) |
| 8 | Release "is the engine linked?" check | **Fixed** (`release.yml:258-259` writes `nm` output to a file first) |
| 9 | Windows Execute takes no lock | **Yes** (#226). The Linux app too. The help pages now say so. Owner decision 5 |
| 10 | Windows Settings Save wipes `settings.json5` | **Yes**, now #232. Not an organiser matter. If it happens, the service has no watch folders and refuses (safe) |
| 11 | "Settled" is 2 seconds of quiet | **Yes** (O17) |
| 12 | Lock-busy retries for ever, noisily; the sweep never retries | **Yes** (O27) |
| 13 | Output folder ignored; nested folders disagree | **Yes, both** (O20, O8) |
| 14 | macOS logs in `/tmp` | **Yes** (O14) |
| 15 | Symlinked or relative watch folders swept once, then ignored | **Yes, proven live** (O21) |
| 16 | A failed write leaves an empty lock file that blocks | **Fixed** (`599d921`; test `an_empty_lock_file_does_not_block`) |
| 17 | Unquoted unit path, unescaped plist, restart loop | **Yes** (O15) |
| 18 | Tests and wording odds and ends | **Not re-checked in full.** Not organiser code. The shared-environment test guard (`test_support::ConfigDirGuard`) is what the organiser tests below use |

---

## 4. Does the #231 fix now cover the organiser's own path?

**Partly.** The organiser calls the same `scan` code, so it gets everything the fix does. But it calls
it with the wrong boundary, and some arrival orders are not covered by anything.

| Situation | Covered? | Why |
| --- | --- | --- |
| The `.cue` itself | **Yes, everywhere** | Rule 3: a file with a disc-image ending is never renamed on its own |
| The `.bin` while it is still arriving under a temporary name, including aria2's `.aria2` companion | **Yes** | It is ignored as a download in progress |
| The rip's log and artwork **in the same folder**, once the `.cue` is there | **Yes** | An incomplete set seals its folder. The gap fix makes that "always, and held back" |
| The rip's files **in a subfolder** (`Scans/`, `Bonus/`), when the organiser handles an event there | **No** (O2) | The boundary is the event folder, so the upward walk never reaches the rip folder. Fixed in c2 by passing the watched folder as the boundary |
| The rip's files when **the `.cue` has not arrived yet** | **No** (O3) | No set exists yet. Fixed in c2 (a folder holding an unpaired disc-image file, or a download in progress, is held back) and backed up in e1 (a folder is not organised while anything in it is still arriving) |
| The opt-in start-up sweep | **Yes** | Its boundary is the watched folder |

---

## 5. Where the organiser's "held back" report goes (decision)

**Decided:** in two places, and not as a desktop notification yet.

1. **The organiser's log.**
   - In a terminal run, that is the screen. For the service it is the log file: stage d3 moves the macOS
     log to `~/Library/Logs/MeedyaManager/organiser.log`; on Linux it is the systemd journal.
   - Each held-back folder is reported **once**, with its reason and the missing file's name, when first
     seen.
   - It is reported again when the reason changes, and once more when it clears ("no longer held back").
     It is not repeated on every scan. (Stage c2.)
2. **A status file that `meedya service status` reads** (stage e3).
   - The organiser keeps `organiser-status.json` in the settings folder.
   - `meedya service status` shows a **"Needs your attention"** list: held-back folders, files left alone
     because they would be renamed again, conflicts, and failed moves. `--json` includes the same list.
   - This is the place somebody looks when the service "isn't doing anything".

**Rejected for now: desktop notifications.**
- A service can run with no desktop session (a Linux user service, or a Mac at the login window).
- Each platform needs its own code (`osascript`, `notify-send`, Windows toasts). None of it can be
  tested in CI.
- Raised as a suggestion in §8.

---

## 6. How every stage is carried out

### 6a. Rules common to all stages

- **Start point.** Cut each stage's worktree (`.claude/worktrees/org-<stage>`) from the working
  branch `claude/musicbrainz-api-migration-7jxszn` **after** `review/round2`, including the gap-fix commit
  and any fixes from Codex round 2, has landed on it.
  - c2 especially must not start before then, because it builds on the gap fix's `disc` and `scan`
    changes.
  - Each later stage starts from the branch head after the previous stage has landed.
- **The safety catch stays off until g.** `ORGANISING_SWITCHED_ON = false` throughout.
  - Tests that need real moves call the organiser's internal functions directly, under
    `test_support::ConfigDirGuard`, as the existing `organise_directory_*` tests do.
  - `run()`-level tests of real organising cannot pass before g, because the switch refuses first.
    Write them against small pure functions instead, as each stage says.
- **Tests first.**
  - Write the stage's tests, run them, and **save the failing output** in each test's doc comment, as
    the project already does (see `watch_organize_with_no_folders_is_a_plain_error`).
  - Then implement.
  - Where a test exercises a brand-new function, "failing" can only mean "does not compile yet". Each
    stage marks which tests those are, and has at least one test that fails *by behaviour* on today's
    code.
- **Gate.** Read every exit code directly, never through `| tail` or `| grep`:
  ```bash
  export PATH="$HOME/.cargo/bin:$PATH"
  cargo fmt --all --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace          # twice; both runs must match
  RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
  cargo deny check                # needed whenever Cargo.toml or Cargo.lock changes
  ```
  - **Format only the files you touched**: `rustfmt --edition 2024 <file>`. Never `cargo fmt --all`.
  - `rust-version` is 1.85 (#234). Clippy flags newer standard-library calls
    (`clippy::incompatible_msrv`). Reuse `LockFile::try_acquire` rather than calling `File::try_lock`
    again. Use `std::path::absolute` (1.79) rather than anything newer.
- **What cannot be checked on this Mac, and must be said in every stage report:**
  - Linux-only code (the `systemctl` calls);
  - Windows-only code (`install_windows`; a dependency's C build needs a Windows C toolchain);
  - `crates/mm-gtk` (excluded from the workspace).
  CI covers them at stage g.
- **Keep mm-gtk building blind.** `RenamePreview` and `RenameSummary` in `mm-core/src/renamer/mod.rs`
  must not change shape, because `mm-gtk` uses them and cannot be compiled here.
- **No changelog edits in stages c1 to e3.** Every stage editing `docs/changelog.md` would make parallel
  worktrees clash. Stage f2 writes one entry for all of them. The orchestrator updates
  `.claude/HANDOFF.md` after each stage.
- **Review.**
  - Sonnet builds; an Opus agent that did not build reviews. Where Opus builds (e1), a *fresh* Opus
    agent reviews, and the handoff records that this reviewer is the same model.
  - Fix, then review again until a round finds nothing real, and record the number of rounds.
  - Codex's allowance is kept for the review of the whole organiser before g.
- **Commit** each stage on its own, as `fix(organiser): … (#180)` (or `fix(service): …`, or
  `docs: …`), as `Salem874`, with the project's closing lines. **Do not push** unless the owner asks.
  Comment on #180 with what landed, and on #231 or #49 where a stage closes part of them.

### 6b. The hand-check harness (every stage)

Every hand check uses **`--dry-run`**, an **empty `MM_CONFIG_DIR` in the scratchpad**, and **scratch
files in the scratchpad**. It never touches the owner's files or settings.

Save each check as a script (for example `hand-c1.sh`) and keep its output. Template:

```bash
#!/bin/bash
# Hand check for stage <X>. Moves nothing: --dry-run throughout, a throwaway
# settings folder and throwaway files, all inside the session scratchpad.
set -u
W="/abs/path/to/.claude/worktrees/org-<stage>"          # this stage's worktree
B="$W/target/debug/meedya"                              # built with: cargo build -p mm-cli
S="<session scratchpad>"
H="$S/hand-<stage>"; rm -rf "$H"; mkdir -p "$H/config" "$H/lib"
export MM_CONFIG_DIR="$H/config"
HR="$(cd "$H" && pwd -P)"          # the real path: macOS reports /private/tmp, not /tmp
cat > "$H/config/settings.json5" <<EOF
{ rename: { template: "<Extension>/<Filename>" }, watch: { folders: ["$HR/lib"] } }
EOF
# … make fixture files with printf; no tagging tool is needed for location-based templates …
find "$H/lib" -type f | sort > "$H/before.txt"
timeout -s INT 60 "$B" --dry-run watch --organize --settle-secs 1 > "$H/out.txt" 2>&1 &
P=$!
for i in $(seq 1 80); do grep -q "Watcher started" "$H/out.txt" && break; sleep 0.25; done
# … create the files that should raise events …
for i in $(seq 1 160); do grep -q "<text you expect>" "$H/out.txt" && break; sleep 0.25; done
kill -INT "$P"; wait "$P"; echo "exit code: $?"
cat "$H/out.txt"
# Prove nothing moved: every file in before.txt must still exist at the same path.
while read -r f; do [ -e "$f" ] || echo "MOVED OR MISSING: $f"; done < "$H/before.txt"
```

- `timeout` is at `/opt/homebrew/bin/timeout` on this Mac.
- After e1, settling takes about **two** settle periods, so wait at least 3 seconds with
  `--settle-secs 1`.
- If the orchestrator waits for a background run, it sets a watchdog
  (`~/.claude/bin/watchdog.sh contains "$H/out.txt" "Watcher stopped" 300`).

---

### Stage c1 — Stop the rename loop

**Fixes:** O1, and the "lock busy" guesswork in O27. The waiting part of O27 comes in e3.
**Model:** Sonnet to build (the specification is precise); Opus to review.

**Files:**
- `crates/mm-core/src/renamer/mod.rs`
- `crates/mm-cli/src/commands/scan.rs`
- `crates/mm-cli/src/commands/watch.rs`

**What to build:**

1. **In `renamer/mod.rs`, one shared planning step.** The name-to-destination code is duplicated in
   `simulate_rename` (`:406-433`) and `simulate_rename_with_rules` (`:511-534`).
   - Extract it into `pub fn destination_for(raw_path: &str, source_ext: Option<&str>, output_dir: &Path,
     sanitize: &SanitizeConfig) -> PathBuf`.
   - Add `pub fn plan_with_rules(ctx: &EvalContext, rules: &[Rule], default_template: &str,
     source_ext: Option<&str>, output_dir: &Path, sanitize: &SanitizeConfig) -> MmResult<PathBuf>`.
     It applies the rules first, then the template, then `destination_for`.
   - `simulate_rename_with_rules` uses it. **No change in behaviour, and no change to `RenamePreview` or
     `RenameSummary`.**
2. **In `scan.rs`, a structured way in for the organiser.**
   - `pub(crate) struct ScanHooks<'a>`. `Default` means exactly today's `meedya scan`. Its fields:
     - `quiet: bool` — print nothing; the caller reports;
     - `leave_repeats_alone: bool`;
     - `guard: Option<&'a mut dyn MoveGuard>`.
     Later stages add `protect_within` (c2) and `stop` (d2).
   - `pub(crate) struct ScanOutcome`, holding:
     - `exit_code`;
     - `lock_busy: bool` and `lock_unavailable: Option<String>`;
     - `moved: Vec<(PathBuf, PathBuf)>`, marked as previews in dry-run;
     - `unchanged: usize`;
     - `left_alone: Vec<LeftAlone { path, reason, detail }>`, where the reasons are
       `WouldRenameAgain`, `KeepsBeingRenamed`, `Conflict` and `Failed`;
     - `held_back` (filled in c2);
     - `disc_folders: Vec<PathBuf>`;
     - `stopped: bool` (d2).
   - `pub(crate) fn run_with(ctx, args, hooks) -> anyhow::Result<ScanOutcome>`. The existing
     `pub fn run` becomes `run_with(…, ScanHooks::default()).map(|o| o.exit_code)`, so `main.rs` and
     every existing test are untouched.
   - **"Would rename again" check.** For each preview that is neither unchanged nor a conflict:
     - build the evaluation context from the *source's* already-read tags, but with the file path set to
       the *planned destination*. Add `build_eval_context_at(source, at, extracted, mode)` next to
       `build_eval_context`;
     - call `plan_with_rules` with the destination's own ending;
     - if the answer differs from the planned destination, flag the preview.
   - **What happens to flagged previews:**
     - **Plain `scan --execute`** still carries them out (a one-off `<Filename>` rename is a legitimate
       thing to do by hand). It prints: *"Note: N files would be renamed again if you ran this again,
       because your template uses the file's own name or folder."* JSON gets `would_rename_again: bool`
       on each preview and a `would_rename_again` count in the summary. The exit code is unchanged.
     - **With `leave_repeats_alone`** (the organiser), flagged previews are **not** carried out. They go
       into `left_alone` as `WouldRenameAgain`.
   - **Before every move**, if `guard` is set, ask `guard.check(source, destination)`. The answer is
     `Move`, `AlreadyDone` (count as unchanged) or `LeaveAlone(reason)`. After a successful move, call
     `guard.record(…)`. The `MoveGuard` trait is declared here.
3. **In `watch.rs`:**
   - Organise through `run_with` with `quiet: true`, `leave_repeats_alone: true` and
     `guard: Some(&mut self.history)`.
   - Decide retries from `outcome.lock_busy`, not from exit code 1.
   - **Report each outcome in a compact form:**
     - one line per moved file (`[HH:MM:SS] Moved <from> → <to>`, or `Would move` in dry-run);
     - one line per left-alone file, **once per file and reason** (keep a set of what has already been
       said);
     - errors as they come.
     With `--json`, write **one compact JSON object per line** (`{"time", "event", "path", "to",
     "reason", "detail"}`), and write the start-up object compactly too, so `watch --json` is a clean
     stream of lines.
   - **`MoveHistory` (implements `MoveGuard`)** remembers the files *this run* has moved.
     - The key is the file's identity: device and inode number on macOS and Linux (read with
       `std::os::unix::fs::MetadataExt` **before** the move); on other systems, the destination path.
     - The value is: last destination, number of moves, first move time.
     - `check` answers `AlreadyDone` when the planned destination equals the last one used.
     - It answers `LeaveAlone(KeepsBeingRenamed)` after **3 moves within an hour**.
     - Entries older than an hour are forgotten at the start of each folder. The clock is passed in, so
       tests need no real waiting.

**Decisions, and what was rejected:**
- **Two guards, not one.**
  - *The prediction* (work out where the file would go from its new home) stops a template-caused loop
    **before the first wrong move**.
  - *The history* catches loops the prediction cannot see:
    - a file system that changes a name after the move (older Mac drives store accented names in a
      different Unicode form; whether that raises a fresh event is **not verified**);
    - two organisers disagreeing;
    - anything unforeseen.
  - Rejected: the history alone, because it lets `01 - song.wav` happen once.
  - Rejected: "never move a file this run already moved", because it would stop a file being re-filed
    after somebody corrects its tags.
- **Plain `scan` still performs one-off renames.** §15 said to leave repeating files alone everywhere,
  with exit code 2. Rejected: that breaks a legitimate manual feature in order to fix a hazard that only
  exists when the same template runs again automatically. §15 itself had deferred the "override" that
  would then have been needed.
- **The organiser still forces "skip"** for conflicts. The numbering bug (#224) is still open.
- **A structured result instead of exit codes.** The organiser's retry decision is currently a guess.
  It must not be.

**Tests to write first** (in brackets: why each one fails today):
1. `watch.rs` `organising_leaves_alone_a_file_whose_new_name_would_change_again` — template
   `x <Filename>`, file `in/song.wav`. After organising `in`, `song.wav` is still there, no `x song.wav`
   exists, and the result lists it as left alone. *(Today it is renamed to `x song.wav`.)*
2. `watch.rs` `organising_twice_moves_nothing_the_second_time` — the same template, organised twice. After
   both passes, no file name contains `x x `. *(Today the second pass makes `x x song.wav`.)*
3. `watch.rs` `organising_is_settled_for_the_default_template` — a tagged WAV: the first pass moves it,
   the second moves nothing. *(This one passes today; it is a guard.)*
4. `scan.rs` `scan_execute_still_does_a_one_off_filename_rename_and_counts_it` — the outcome shows the
   move, with `would_rename_again` equal to 1. *(Compile failure: `run_with` does not exist yet.)*
5. `renamer` `plan_with_rules_agrees_with_simulate` — for the templates the existing tests already use.
   *(Guard.)*
6. `watch.rs` `MoveHistory` tests, with an injected clock:
   - `the_same_destination_twice_counts_as_already_done`;
   - `a_file_moved_three_times_within_an_hour_is_left_alone`;
   - `entries_older_than_an_hour_are_forgotten`.
   *(Compile failure.)*
7. `watch.rs` `a_busy_lock_is_recognised_from_the_result_not_the_exit_code` — hold the write lock
   inside the test, as `scan_execute_refuses_while_another_process_holds_the_lock` already does
   (`LockFile::try_acquire(&LockFile::default_path(), "another test")` under `ConfigDirGuard`).
   Organise, and check `lock_busy` is true. Then a folder that has vanished must *not* count as busy.
   *(Compile failure.)*

**Hand check** (harness in §6b):
- **(a)** `"$B" --dry-run scan "$H/lib" --execute --template 'x <Filename>'`, with `lib/song.wav`
  (`printf x > …`). The preview shows `x song.wav` **and** the "would be renamed again" note. Repeat with
  `--json`: `would_rename_again: true`.
- **(b)** Settings template `x <Filename>`. Start the dry-run organiser, then create `lib/in/song.wav`.
  The output shows it left alone, "would be renamed again", and never "Would move … song.wav".
- **(c)** Settings template `<Extension>/<Filename>`, same file. The output shows
  `Would move …/lib/in/song.wav → …/lib/wav/song.wav`.
- Nothing moved in any of the three.

**Done when:** the gate is clean, the tests are shown failing first, the hand check output is saved, and
a review round finds nothing.

---

### Stage c2 — Keep rips and arriving downloads whole on the organiser's path; report held-back folders

**Fixes:** O2, O3, and the log half of O4.
**Model:** Sonnet; Opus review.

**Files:**
- `crates/mm-cli/src/commands/scan.rs`
- **new** `crates/mm-core/src/hold.rs` (and its `mod` line in `crates/mm-core/src/lib.rs`)
- `crates/mm-cli/src/commands/watch.rs`

**What to build:**

1. **`ScanHooks.protect_within: Option<&Path>`.** When set, `scan` passes it to `partition_for_scan`
   instead of the scanned folder. The organiser sets it to the watched folder that owns the event.
   Because the gap fix already walks up to that boundary, the rip folder above a `Scans/` or `Bonus/`
   subfolder is now examined.
2. **`mm-core/src/hold.rs`**, "folders whose files must not move yet". It holds one function,
   `pub fn folders_to_hold(files: &[PathBuf], boundary: &Path) -> MmResult<Vec<HeldFolder>>`, which looks
   at every folder between each file and the boundary. It holds a folder back for two reasons:
   - **`DownloadArriving { names }`.** The folder directly holds a file for which
     `watcher::is_download_in_progress` is true (aria2-aware after the gap fix).
     - The folder's own files are held, **and its subfolders too, unless the folder is the boundary
       itself**. At the boundary only its own direct files are held.
   - **`UnpairedDiscFile { names }`.** The folder directly holds a file with a disc-image ending that is
     **not a member of any set** from `disc::find_image_sets`. The usual case is a music-CD `.bin` whose
     `.cue` has not arrived.
     - It applies only when the folder has no audio or video of its own, and is **not** the boundary.
     - The folder and its subfolders are held.

   `HeldFolder { dir, reason, include_subfolders }` and `pub fn is_held(file, held) -> bool` complete the
   module.
3. **In `scan.rs`:**
   - After `partition_for_scan`, call `folders_to_hold(files, boundary)` and remove held files from the
     loose list.
   - Merge those folders with the gap fix's held-back disc folders (`missing_files` not empty) into
     `outcome.held_back: Vec<HeldBack { folder, reason, detail, files_held }>`. The reasons are
     `IncompleteRip`, `DownloadArriving` and `UnpairedDiscFile`.
   - Human output: extend the gap fix's "Held back" section with the two new reasons, each with a plain
     sentence and what to do. For a download: "…still arriving; run this again when it has finished".
     For an unpaired file: "Album.bin has no cue sheet beside it yet; if none is coming, move the `.bin`
     somewhere else".
   - JSON: add a `held_back` array with `folder`, `reason` (a fixed word), `detail` and `files_held`.
4. **In `watch.rs`:**
   - Pass `protect_within`.
   - Report each held-back folder **once per (folder, reason)**, again when the reason changes, and
     "no longer held back" when it clears.
   - Report complete disc folders once: "left as it is — moving whole disc folders is not built yet
     (#217)".
   - Keep the de-duplication state inside `Organiser`, so a restart reports again. That is useful: a
     restart is when somebody is likely to be looking.

**Decisions, and what was rejected:**
- **Boundary = the watched folder that owns the event.** Rejected: scanning from the watched folder
  every time. That re-plans the whole library on every event.
- **Downloads hold their folder, but a download sitting directly in the watched folder holds only that
  folder's own files.** Otherwise one browser download in a watched `~/Downloads` would freeze everything
  beneath it for as long as the browser is busy. Rejected: holding nothing (today: companions move).
- **A lone disc-image file holds its folder, except at the boundary.**
  - Rejected: changing Rule 4 in `find_image_sets` so that a lone `.bin` becomes an "image". That would
    also change `detect_disc_folder`, which other callers use, and would make any firmware `.bin` a disc.
  - Rejected: holding at the boundary too. A router-firmware `.bin` downloaded into a watched
    `~/Downloads` would then freeze the entire watched folder.
  - Cost accepted: a non-media folder with a stray `.bin` *inside* a watched folder is held back. It is
    announced, so it is fixable.
  - **Note the difference from the gap fix:** a stray **`.cue`** at the root *does* freeze the root
    (the gap fix's "always sealed" decision). A `.bin` is a far more common file outside music, which is
    why it is treated more gently.
- **The new module lives in `mm-core`, not in `disc`.** Downloads are not disc knowledge, and a separate
  file does not clash with the gap fix's large changes to `disc/mod.rs`.

**Tests to write first:**
1. `watch.rs` `an_event_in_a_rip_subfolder_leaves_the_artwork_in_the_rip` — a complete rip (`Album.cue`
   plus a sync-pattern `Album.bin`) with `Rip/Scans/cover.jpg`. Organise folder `Rip/Scans`, owned by the
   root, with template `<Extension>/<Filename>`: `cover.jpg` stays. *(Today it moves, even with the gap
   fix.)*
2. `watch.rs` `an_event_in_an_arriving_rips_subfolder_leaves_the_bonus_track` — a `.cue` naming a missing
   `.bin`, plus `Rip/Bonus/track.mp3`. It stays. *(Moves today.)*
3. `scan.rs` `a_folder_with_a_download_still_arriving_is_held_back` — `Rip/Album.bin.crdownload` and
   `Rip/Album.log`, no cue, scanning the parent with `--execute`. The log stays, and `held_back` names the
   download. *(The log moves today.)*
4. `scan.rs` `a_download_directly_in_the_scan_folder_holds_only_that_folders_files` —
   `root/movie.mkv.part`, `root/a.wav`, `root/Sub/b.wav`. `b.wav` is organised; `a.wav` is held.
5. `scan.rs` `a_lone_bin_without_its_cue_holds_its_folder` — `Rip/Album.bin` (zeros) and `Rip/Album.log`.
   The log stays; the reason is `UnpairedDiscFile`. *(Moves today.)*
6. `scan.rs` `a_stray_bin_directly_in_the_scan_folder_does_not_freeze_the_library` — `root/firmware.bin`
   and `root/Artist/a.wav`. `a.wav` is organised. *(Guard.)*
7. `scan.rs` `music_beside_a_stray_bin_is_still_organised` — FLACs plus a `.bin` in one folder. The FLACs
   are organised. *(Guard, matching `folder_with_flacs_and_stray_bin_…`.)*
8. `scan.rs` `an_aria2_download_holds_its_folder` — `Film/movie.mkv`, `Film/movie.mkv.aria2` and
   `Film/poster.jpg`. The poster stays.
9. `watch.rs` `a_held_back_folder_is_reported_once_and_again_when_it_clears` — reporter unit test.
   *(Compile failure.)*

**Hand check (dry-run):**
- Fixtures: `lib/Rip/Album.cue` (containing `FILE "Album.bin" BINARY`), `lib/Rip/Album.bin.part` and an
  empty `lib/Rip/Scans/`.
- After "Watcher started", create `lib/Rip/Scans/cover.jpg`. Expect one "Held back … Rip …" line naming
  `Album.bin`, and **no** "Would move … cover.jpg".
- Then create `lib/DL/Album.bin` (zeros) and `lib/DL/Album.log`. Expect "Held back … no cue sheet beside
  it yet".
- Then `lib/Film/movie.mkv`, `lib/Film/movie.mkv.aria2` and `lib/Film/poster.jpg`. Expect the poster to
  be held.
- Also run `"$B" --dry-run --json scan "$H/lib" --execute` and save it. `held_back` lists all three with
  the right reasons.

---

### Stage d1 — Existing files left alone unless asked; an honest question; no unattended start without `--yes`

**Fixes:** O5, O6, O7, O8.
**Model:** Sonnet; Opus review.
**Files:** `crates/mm-cli/src/commands/watch.rs` only.

**What to build:**

1. **A new flag, `--organize-existing`** (clap `requires = "organize"`). Without it there is **no
   sweep**. The service definitions from d3 never include it.
2. **The sweep, rebuilt as a walk over folders.** For each watched folder:
   - collect every folder beneath it:
     - do **not** follow symlinked folders (use `DirEntry::file_type()`);
     - skip hidden folders and Safari `.download` folders;
     - **skip any subtree that belongs to a deeper watched folder**, which is swept under its own root;
   - make the complete list **before** anything moves;
   - organise each folder on its own, through the same per-folder function the events use, owned by its
     root (so c2's protection applies).
3. **Two passes.**
   - **Preview pass** (quiet, dry). Totals: files that would move, folders involved, held back, left
     alone. Print: *"Existing files: N files in M folders would be moved; K left alone (…)."*
   - **Then:**
     - under `--dry-run`, stop there and carry on watching;
     - on a terminal, ask *"Move them now? [y/N]"*;
     - with `--yes`, go ahead;
     - with no terminal and no `--yes`, refuse (see 4).
4. **A consent rule for the organiser:** `organise_consent(yes, stdin_is_terminal) -> Consent`.
   - Terminal without `--yes`: **Ask**.
   - No terminal and no `--yes`: **Refuse**, exit code 1: *"Nobody is here to confirm, so nothing will be
     moved. The background service passes --yes; if you meant to run this unattended, add --yes."*
   - `scan` keeps its own rule, because scripts rely on it.
5. **An honest start question:** *"MeedyaManager will watch these folders and, whenever a new or changed
   file has settled, move it into place under <output>. Files already there are left alone — add
   --organize-existing to organise them too. Continue? [y/N]"* (e2 fills in the real output folder.)
6. The watcher still starts before the sweep, as now (`watch.rs:626-631`), so arrivals during the sweep
   are not missed.

**Decisions, and what was rejected:**
- **The sweep is opt-in** (adopted in §15 and announced on #180).
  - Rejected: a sweep in the service, which reorganises the library at every login.
  - Rejected: a question with no numbers. The owner's rule is to say what will happen.
- **Folder by folder, not one recursive `scan`.** One recursive scan cannot be stopped part-way (d2),
  and it disagrees with the events about which watched folder owns a file (O8).
- **No terminal and no `--yes` means refuse.** This closes the trap of old service definitions without
  `--yes`. An old Linux unit with `Restart=on-failure` will then restart every 5 seconds, refusing each
  time. That is noisy but harmless; d3's status check tells the user to reinstall.

**Tests to write first:**
1. `existing_files_are_left_alone_without_organize_existing` — build an `Organiser`, run `event_loop` with
   a channel whose sender is already dropped, and check the file is untouched. *(Today the sweep moves
   it.)*
2. `organize_existing_moves_existing_files` — the same, with the flag. The file is moved.
3. `the_sweep_leaves_an_inner_watched_folder_to_its_own_root` — roots `[m, m/In]`, file `m/In/x.wav`,
   template `<Extension>/<Filename>`. It lands at `m/In/wav/x.wav`, not `m/wav/x.wav`. *(Today:
   `m/wav/x.wav`.)*
4. `the_sweep_never_follows_a_symlinked_folder` — `root/link` points to `../elsewhere`, which holds
   `x.wav`. It is untouched.
5. `organise_consent_refuses_without_a_terminal_or_yes`, with all four combinations. *(Compile failure.)*
6. `the_sweep_counts_before_it_asks` — a fixture with 2 movable files in 2 folders and 1 held folder.
   The counts are exact. *(Compile failure.)*

**Hand check (dry-run):**
- Put `lib/a/one.wav` and `lib/b/two.wav` in place before starting.
- `"$B" --dry-run watch --organize`: the output says existing files are left alone, with **no** "Would
  move" lines for them.
- Then `"$B" --dry-run watch --organize --organize-existing`: the totals line "2 files in 2 folders", and
  two "Would move" lines.
- Nested: settings folders `["$HR/lib", "$HR/lib/In"]` and a file `lib/In/x.wav`. The sweep preview shows
  `lib/In/wav/x.wav`.

---

### Stage d2 — Ctrl+C that works; interrupted copies never leave half a file

**Fixes:** O9, O10.
**Model:** Sonnet; Opus review.

**Files:**
- `crates/mm-cli/src/commands/watch.rs`
- `crates/mm-cli/src/commands/scan.rs`
- `crates/mm-core/src/renamer/mod.rs`

**What to build:**

1. **Stopping.**
   - `run` creates a stop flag (`Arc<AtomicBool>`) and a small task. On the **first** Ctrl+C it sets
     the flag and prints (to the error stream) *"Stopping after the file being moved now… press Ctrl+C
     again to stop immediately."* On the **second**, it prints *"Stopping immediately."* and calls
     `std::process::exit(130)`.
   - Everything else moves into `run_until(ctx, args, stop)`, so tests can stop it without signals.
2. **Where the flag is checked:**
   - between folders (event loop and sweep);
   - in `scan`, through a new `ScanHooks.stop: Option<&AtomicBool>`, between files, in both the
     tag-reading pass (`extract_all`, `scan.rs:193-210`) and the move pass (`execute_previews`,
     `:423-500`).
   A stop leaves the remaining files untouched and sets `outcome.stopped`.
3. **"Watcher stopped"** is printed only after the loop has returned and the watcher has been dropped.
   The exit code is 0 after the first Ctrl+C.
4. **Copies go through a hidden temporary name.** In `execute_rename_with`'s copy branch
   (`renamer/mod.rs:665-674`):
   - copy to `.<final name>.meedya-partial-<process number>` in the destination folder;
   - check the destination is still free, then rename the temporary file into place;
   - on any error, delete the temporary file and report.
   The name starts with a dot, so scans and the watcher already ignore it. Put the logic in a helper
   (`copy_into_place(src, dst, copier)`) so the failure path can be tested.

**Why exiting at the second press is safe** (the check §15 asked for, done):
- A move is one `rename` system call (`renamer/mod.rs:677`): all or nothing.
- There is **no** copy-then-delete fallback for moves between drives. `rename` simply fails with an
  error, and e2 refuses that set-up before starting.
- Copy mode, after step 4, leaves either nothing or a hidden leftover, never a half file under a real
  name.

**Decisions, and what was rejected:**
- **Check between files, not only between folders.** §15 planned between folders only. Rejected: a large
  folder can take minutes, which is exactly why Ctrl+C felt dead.
- **Stopping gracefully on the service's stop signal is not built.** Every step is now all-or-nothing, so
  an immediate stop is safe. (§15's "Not now".)
- **Leftover `.meedya-partial-…` files after a power cut are not cleaned up automatically.** Doing that
  safely needs to know no copy is running. Documented in f2; suggestion in §8.

**Tests to write first:**
1. `scan.rs` `a_stop_request_before_moving_moves_nothing` — flag set before the call. Nothing moves;
   `stopped` is true. *(Compile failure.)*
2. `scan.rs` `a_stop_request_after_the_first_move_leaves_the_rest` — a test `MoveGuard` that sets the
   flag in `record()`. Exactly one of three files moves.
3. `renamer` `an_interrupted_copy_never_leaves_a_half_file_under_the_final_name` — a copier that writes
   half, then fails. The destination does not exist and no temporary is left. *(Compile failure.)*
4. `renamer` `copy_mode_still_refuses_an_existing_destination`. *(Guard.)*
5. `watch.rs` `run_until_returns_promptly_when_stopped` — a dry-run context (allowed while switched off),
   with a thread that sets the flag after 200 ms. It returns SUCCESS within 5 seconds.
   *(Compile failure.)*

**Hand check (dry-run):**
- Generate 3,000 small `.wav` files in 300 folders under `lib`, with a shell loop.
- Run `"$B" --dry-run watch --organize --organize-existing`. When the sweep's preview has started, send
  **one** `kill -INT`. The output shows "Stopping after…", then "Watcher stopped", and the exit code is 0,
  all within a few seconds.
- Run again and send **two** `kill -INT`, a second apart. Exit code 130.
- Nothing moved.
- Copy mode cannot be hand-checked without real copies; it is covered by tests 3 and 4 only. Say so in
  the report.

---

### Stage d3 — The background service: honest install, safe definitions *(parallel track)*

**Fixes:** O11 to O16, and it spots definitions left by older builds.
**Model:** Sonnet; Opus review.

**Files:**
- `crates/mm-core/src/service.rs`
- `crates/mm-cli/src/commands/service_cmd.rs`

**What to build:**

1. **Pure text builders, compiled on every system** so the tests run on this Mac:
   `pub fn systemd_unit(spec: &ServiceSpec) -> String` and
   `pub fn launchd_plist(spec: &ServiceSpec) -> String`.
   - `ServiceSpec` holds: the program; an optional `--config` file (kept if the user passed the global
     `--config` to install); an optional `MM_CONFIG_DIR` (kept if set when installing); and the log file
     (macOS).
   - The program's arguments are always `[--config <file>] watch --organize --yes`, **never**
     `--organize-existing`.
2. **systemd unit:**
   - every `ExecStart=` argument is quoted by systemd's rules: wrap in `"`; escape `\` and `"`; double
     `%` and `$`;
   - `Restart=on-abnormal`, so it restarts after a crash but not after a deliberate refusal or stop;
   - in `[Unit]`, `StartLimitIntervalSec=300` and `StartLimitBurst=5`;
   - `Environment="MM_CONFIG_DIR=…"` when set;
   - drop `After=network.target`, which means nothing in a per-user unit. Say so in a comment.
3. **launchd plist:**
   - XML-escape every inserted value (`&`, `<`, `>`, `"`, `'`);
   - `StandardOutPath` and `StandardErrorPath` both point to
     `~/Library/Logs/MeedyaManager/organiser.log`, with the folder created at install, readable by the
     owner only;
   - `EnvironmentVariables` for `MM_CONFIG_DIR` when set;
   - keep `KeepAlive {Crashed: true}`; set `ThrottleInterval` to 30.
4. **macOS commands:**

   | Action | Command |
   | --- | --- |
   | install | `launchctl bootstrap gui/<uid> <plist>` (**checked live: fails with exit 5**; `load` returned 0) |
   | uninstall | `launchctl bootout gui/<uid>/<label>` |
   | status | `launchctl print gui/<uid>/<label>` (**exit 113 = not known**), plus "is the plist file there?" |
   | start | `launchctl kickstart gui/<uid>/<label>` |
   | stop | `launchctl kill SIGTERM gui/<uid>/<label>` |

   Read `<uid>` from the owner of the home folder (`MetadataExt::uid()`), so no `unsafe` code is needed.
5. **Linux commands:** `daemon-reload`, then `enable --now`, so it starts now as well as at login.
   Status: no unit file means **NOT INSTALLED**; otherwise use `is-active`.
6. **The install message says what happened:** *"Installed and started. It will also start every time
   you log in. Log: <path>."*
   - On Linux, add: *"It runs only while you are logged in, unless you run `loginctl enable-linger`."*
   - Mention the kept `--config` or `MM_CONFIG_DIR`, if any.
7. **Checks before anything is written**, placed *before* the switched-off refusal so they can be tested
   now: at least one watch folder in settings, otherwise refuse (exit 1). Stage e2 replaces this with
   the full organiser pre-flight.
8. **`--dry-run`** prints the full unit or plist text and the path it would be written to.
9. **`status`** compares the installed definition with what this build would write. If they differ, it
   says *"This service was set up by an older build — run `meedya service install` again to update
   it."* It names a missing `--yes` specifically.
10. **Split `install`** into `#[cfg(windows)] fn install_windows` and `#[cfg(not(windows))] fn
    install_unix`. This also removes the Windows `needless_return` Clippy failure noted in §15(d).
    Windows keeps refusing (the Task Scheduler route).
11. **Keep the switched-off refusal** (`service_cmd.rs:159-169`) until stage g.

**Decisions, and what was rejected:**
- **`launchctl load`/`unload`/`start`/`stop` → `bootstrap`/`bootout`/`kickstart`/`kill`**, because `load`
  reports success on failure (proven).
- **`Restart=on-failure` → `on-abnormal`**, because a refusal is deliberate and should stay stopped, with
  the reason in the log and in `status`.
- **`/tmp` logs → `~/Library/Logs`**, because `/tmp` is shared by every account.
- **Install starts the service.** On macOS, `RunAtLoad` starts it anyway at bootstrap, and one behaviour
  on both systems is easier to explain. With the sweep opt-in (d1), starting touches no existing file.
- **Keep `--config` and `MM_CONFIG_DIR` in the definition.** Otherwise the service would read different
  settings, and take its write lock in a different folder, from the terminal that installed it, and the
  two would not keep out of each other's way. This also makes the optional real install trial in g
  safe.

**Tests to write first:**
1. `the_systemd_unit_quotes_a_path_with_spaces_and_percent_signs`. *(Compile failure on this Mac: the
   builder is Linux-only today.)*
2. `the_launchd_plist_escapes_an_ampersand`. On macOS, also write it to a temporary file and run
   `plutil -lint`; skip if `plutil` is missing.
3. `the_launchd_plist_logs_to_library_logs_not_tmp`.
4. `the_service_never_sweeps_existing_files` — the arguments contain `--yes`, not `--organize-existing`.
5. `macos_status_says_not_installed_when_launchd_does_not_know_it`. macOS only; skip if the real plist
   exists. *(Behavioural failure today: it returns Stopped.)*
6. `install_refuses_when_no_watch_folders_are_set`, through the dry-run path. *(Today dry-run returns 0.)*
7. `status_spots_a_definition_from_an_older_build`.
8. `the_install_message_says_it_has_started`.

**Hand check** (writes nothing outside the scratchpad, installs nothing):
- First run `launchctl print gui/$(id -u)/com.mwbm.meedyamanager`. It should say it is not known. **If
  a real service is installed, skip the status part and report that.**
- With `MM_CONFIG_DIR="$H/config"` and a settings file naming one watch folder:
  - `"$B" --dry-run service install --bin-path "$H/My Apps & Co/meedya"` prints a plist. Save it to
    `$H/test.plist` and run `plutil -lint`. It must pass. Check that the `&` is escaped, the log path is
    in `~/Library/Logs/MeedyaManager/`, `--yes` is present, and `MM_CONFIG_DIR` is kept.
  - `"$B" service status` says **NOT INSTALLED**.
  - With no watch folders in settings, `"$B" --dry-run service install` refuses with exit code 1.

---

### Stage e1 — "Settled" means really finished; folders moved in whole; hidden folders

**Fixes:** O17, O18, O19.
**Model:** **Opus** to build. This is the one genuinely fiddly piece: a queue driven by time. A fresh
Opus agent reviews it, and the handoff records that the reviewer is the same model, so Codex should look
hard here.

**Files:**
- `crates/mm-cli/src/commands/watch.rs`
- `crates/mm-core/src/watcher/mod.rs`

**What to build:**

1. **A folder-level settle queue**, replacing `Pending` and `take_settled`. It is pure: the clock and the
   folder listing are passed in.
   - `record(event, now, is_dir)`:
     - a file event queues its folder;
     - a folder that appears (created, or moved in) is queued as a **whole subtree**;
     - a deletion or rename-from also touches its folder;
     - every event resets that folder's quiet timer and discards its last listing.
   - `ready(now, list)` returns a folder when **all** of these hold:
     1. no event for `settle`;
     2. a listing (name, size, modified time of each direct file; of every file below it for a subtree)
        equals one taken at least `settle` earlier;
     3. no entry in it is still downloading (`is_download_in_progress`);
     4. the folder still exists. A vanished folder is dropped.

     Otherwise it stores the new listing and waits.
   - `requeue(folder, now, not_before)` is used by the retries in e3.
   - A ready subtree takes any queued folders inside it.
2. **Default `--settle-secs` becomes 30.** A folder is organised about a minute after it goes quiet.
3. **Ready subtrees are organised folder by folder**, with d1's walk and the same per-folder function
   (so c2's protection applies).
4. **The organiser ignores events** whose path, below the watched folder, passes through a hidden folder
   (any part starting with `.`) or a Safari `.download` folder. This matches the sweep.
5. **In `watcher/mod.rs`, in the event callback:** extension filters (`include_extensions` and
   `exclude_extensions`) apply to **files only**. A path that is a directory (checked with
   `path.is_dir()` at event time) passes, unless its name is hidden or it is a Safari `.download` folder.
   Put this in a helper, `should_ignore_event(path, config)`, and leave `should_ignore` itself unchanged
   for scans.

**Decisions, and what was rejected:**
- **Folder-level, two identical listings, 30 seconds.** Adopted in §15.
  - Rejected: per-file timers (today's approach; a folder is organised while its other files are still
    copying).
  - Rejected: watching which programs have a file open. It is not portable, and needs extra permissions
    on macOS.
  - Rejected: a settings-file setting. It would have to be added to the shipped settings and schema
    (the drift guard from #211), and it is a property of one run.
- **What this cannot catch, stated plainly:** a download tool that writes under the final name without
  a temporary name, then pauses for more than a minute. Mitigated by c2 and documented in f2: "set your
  download tool to use an incomplete-downloads folder or a temporary name".

**Tests to write first:**
1. `a_folder_moved_in_whole_is_organised_including_its_subfolders` — feed a `Created(dir)` for a folder
   containing `CD1/a.wav` and `CD2/b.wav`, drive the queue past settling, organise. Both are organised.
   *(Today only the parent's own files are scanned; written against today's `record_event` and
   `organise_settled`, it fails by behaviour.)*
2. `a_file_arriving_under_a_hidden_folder_is_never_organised` — an event at
   `root/.Trashes/501/x.wav`. Nothing moves. *(Today it is organised; behavioural.)*
3. `the_default_settle_time_is_thirty_seconds` — parse `WatchArgs` through a small clap `Parser`
   wrapper. *(It is 2 today.)*
4. `a_file_that_keeps_growing_is_not_ready`, `a_folder_with_a_download_in_progress_is_not_ready`,
   `a_folder_that_vanished_is_dropped`, `an_event_restarts_the_wait`. Injected clock and listings.
   *(Compile failure.)*
5. `watcher` `a_directory_passes_the_include_filter` and
   `a_file_without_a_listed_ending_is_still_ignored`. *(Compile failure; the second is a guard.)*

**Hand check (dry-run, `--settle-secs 2`):**
- **(a)** Prepare `$H/src/Album/CD1/a.wav` and `$H/src/Album/CD2/b.wav`. After start, `mv "$H/src/Album"
  "$H/lib/"`. Expect "Would move" for **both**.
- **(b)** Settings `include_extensions: ["wav"]`, then repeat (a). Both still appear.
- **(c)** A loop appending one byte per second to `lib/slow.wav` for 8 seconds. There must be no "Would
  move … slow.wav" until at least 4 seconds after the last byte.
- **(d)** `mkdir -p lib/.Trashes/501 && printf x > lib/.Trashes/501/x.wav`. It is never mentioned.

---

### Stage e2 — Where files go, and when not to move at all

**Fixes:** O20 to O26, and O30.
**Model:** Sonnet; Opus review.
**Needs:** d3 landed (install calls the new pre-flight).

**Files:**
- `crates/mm-cli/src/commands/watch.rs`
- `crates/mm-cli/src/context.rs` (and `main.rs` only if `CliContext::build`'s signature changes)
- `crates/mm-core/src/config/mod.rs`
- `crates/mm-cli/src/commands/service_cmd.rs`

**What to build:**

1. **Output folder:** `rename.output_dir` when set; otherwise the watched folder that owns the file
   (today's behaviour). No new command-line flag.
2. **`organise_preflight(ctx, folders) -> Result<Preflight, Refusal>`.** Used by `watch --organize`
   (including `--dry-run`, so a preview predicts a real run exactly) and by `service install`:
   1. **Settings file present but unreadable → refuse**, naming the file and the error.
      `CliContext` records the load failure (a new `settings_problem: Option<String>` field). Other
      commands keep today's fallback, with a clearer warning.
   2. **Each watch folder** must exist and be a folder, and is resolved to its **real path**: on
      macOS and Linux with `std::fs::canonicalize`; on Windows with `std::path::absolute`, which avoids
      `\\?\` paths that would not match the watcher's events. The real paths are given to the watcher
      too, so events and folders match (O21, proven live).
   3. **Output folder, if set:**
      - it must be a full path;
      - it **must already exist**: *"The output folder … was not found — is its drive connected?
        Nothing has been moved."* The organiser never creates it, because on a Mac a missing
        `/Volumes/Media` would be created on the start-up disk;
      - it must be **on the same drive** as every watch folder, unless copy mode is on (on macOS and
        Linux compare `MetadataExt::dev()`; on Windows compare the drive prefix): *"… is on a different
        drive from …; moving between drives is not supported. Choose an output folder on the same
        drive, or switch on copy mode."*
   4. **The write lock can be taken**: take it and let it go at once. Otherwise refuse with
      `lock_unavailable_message`.
3. **Before each folder** is organised:
   - **If Test Mode is on** (`mm_core::test_mode::is_enabled()`), move nothing. Say once: *"Test Mode is
     on, so the organiser is only watching. Files will be organised once Test Mode is switched off."*
     Keep the folder queued.
   - **If the output folder has gone** (drive unplugged mid-run), hold everything and say so once.
4. **`watch.recursive`** is honoured: recursion is off if the setting **or** `--no-recursive` says so.
5. **`dry_run: true` in settings** makes every command preview-only. `CliContext.dry_run` = the flag
   **or** the setting. It can only make things safer, like #242.
6. **`~` in settings paths.** In `config/mod.rs`, at load time, a leading `~/` (or a bare `~`) in
   `watch.folders`, `rename.output_dir` and `logging.file` is replaced with the home folder
   (`dirs::home_dir()`). A `~` anywhere else in a path is left as it is.
7. **`service install`** calls `organise_preflight` in place of d3's simple watch-folder check.

**Decisions, and what was rejected:**
- **Never create the output folder; refuse between drives.** "Refuse when unsure", and one clear message
  up front instead of an error for every file.
- **Test Mode pauses the organiser** (owner decision 2). The apps already refuse to rename in Test Mode,
  and the organiser runs with nobody watching. Rejected: dropping the queued folders, which would lose
  them because there is no automatic sweep.
- **Expand `~` in the settings loader, not only in the organiser.** `scan --execute` has the same bug
  today, and the shipped example file uses `~`. One place fixes both. **File a new issue for this first,
  and one for `dry_run`** (§8), so the commit can name them.

**Tests to write first:**
1. `the_organiser_uses_the_configured_output_folder`. *(Today: under the watched folder.)*
2. `a_missing_output_folder_is_refused_not_created`. *(Today it would be created.)*
3. `a_relative_output_folder_is_refused`.
4. `events_under_a_symlinked_watch_folder_are_organised` — make an explicit symlink to a temporary folder
   and give the symlink as the root; the event arrives under the real path. *(Today it is skipped.)*
5. `an_unreadable_settings_file_stops_the_organiser`. *(Today defaults are used.)*
6. `dry_run_in_settings_makes_scan_preview_only`. *(Today it moves.)*
7. `a_tilde_in_settings_paths_means_the_home_folder`. *(Today it is literal.)*
8. `the_organiser_moves_nothing_while_test_mode_is_on`, using `TestModeEnvGuard`. *(Today it moves.)*
9. `watch_honours_the_recursive_setting`.
10. `an_output_folder_on_another_drive_is_refused_unless_copying` — a pure comparison with injected
    device numbers.

**Hand check (dry-run):**
- Settings `output_dir: "$HR/out"`, with `out` created. "Would move" lines point into `out/`.
- Delete `out`. The start is refused with the "not found" message, exit code 1, and `out` has **not**
  been created.
- Settings folder given as the scratchpad's **`/tmp/...`** form (the symlink). Events are organised
  ("Would move" appears). This is the live proof of O21, reversed.
- `MM_TEST_MODE=true`: the "only watching" message, and no "Would move".
- `settings.json5` containing `{ rename: `: the start is refused, naming the file.
- **Not hand-checked:** `dry_run: true` in settings. Checking it would mean running *without*
  `--dry-run`, which the hand-check rule forbids. It is covered by test 6 only. Say so.

---

### Stage e3 — Reporting and staying power

**Fixes:** the status half of O4, and O27, O28, O29. Also switches off colour codes in log lines when
they are not going to a terminal.
**Model:** Sonnet; Opus review.
**Needs:** d3 landed.

**Files:**
- `crates/mm-cli/src/commands/watch.rs`
- **new** `crates/mm-cli/src/commands/organiser_status.rs` (and its line in `commands/mod.rs`)
- `crates/mm-cli/src/commands/service_cmd.rs`
- **new** `config/schemas/organiser-status.schema.json`
- `crates/mm-cli/src/main.rs` (`with_ansi(stderr is a terminal)` on the two log set-ups, `:154-156` and
  `:196-198`)

**What to build:**

1. **One real organiser at a time.** An `organiser.lock` in the settings folder, taken with
   `LockFile::try_acquire(path, "meedya watch --organize")` and held for the whole run. A second real
   organiser refuses (exit 1): *"Another MeedyaManager organiser is already running (process N, started
   HH:MM). Nothing has been moved."* Previews do not take it.
2. **`organiser-status.json`** in the settings folder, written by real organisers only.
   - Written as a temporary file, then renamed into place, so a reader never sees half a file.
   - Written at start, after each folder, when the attention list changes, and at stop.
   - Fields:
     - `format_version`;
     - `process_id`;
     - `started_at`, `updated_at` and `stopped_at` (null while running);
     - `watched_folders` and `output_folder`;
     - `paused` (null, `"test_mode"`, `"lock_busy"` or `"output_missing"`);
     - `moved_total`;
     - `attention`: a list of `{kind, path, detail, since}`, capped at 200 entries, with a
       `more_not_shown` count.
3. **A JSON Schema** in `config/schemas/organiser-status.schema.json`: a `description` on every
   property, required fields marked, formats given for times and paths. Plus a drift test modelled on
   `settings_schema_properties_match_appconfig` (`mm-core/src/config/mod.rs:1805`). This follows the
   owner's rule that every JSON file has a schema, checked by a test.
4. **`meedya service status`** shows the service state, then *"Organiser: running since … (process N)"*
   or *"last ran …, stopped …"*, then **"Needs your attention:"** and the list. `--json` carries the same
   fields. Exit codes are unchanged.
5. **A busy write lock.** Wait 30 seconds, doubling each time to a limit of 10 minutes. Print one
   message when a busy spell starts and one when it ends. The sweep uses the same retry. A lock that
   *cannot* be taken is reported once and retried on the same schedule.
6. **Conflicts and failed moves** are not retried, because waiting will not fix them. Each is recorded
   once in the log and in `attention`.

**Decisions, and what was rejected:**
- **Status file plus `service status`**, as decided in §5.
  - Rejected for now: desktop notifications.
  - Rejected: a database, which is #220's job.
- **Previews write no status file**, so a preview cannot overwrite the report of a real service running
  at the same time.

**Tests to write first:**
1. `a_second_organiser_refuses_while_one_is_running`. *(Compile failure.)*
2. `the_status_file_matches_its_schema` — the drift test.
3. `the_status_file_is_replaced_whole` — no temporary file is left behind, and the result parses.
4. `lock_waits_grow_to_ten_minutes_and_are_reported_once`.
5. `a_busy_lock_during_the_sweep_is_retried_not_skipped` — hold the write lock with injected zero waits,
   release it after the first try, and the file is organised. *(Today the sweep never retries.)*
6. `service_status_lists_what_needs_attention` — the text rendering.
7. `a_conflict_is_reported_once_and_not_retried`.

**Hand check:**
- Before stage g, a real organiser cannot run, so the *writing* of the status file is covered by tests
  only. Say so.
- What can be checked:
  - `"$B" service status` with an empty `MM_CONFIG_DIR` shows no organiser section;
  - put a hand-written example `organiser-status.json` (with one held-back folder) in `$H/config`. Then
    `"$B" service status` and `"$B" --json service status` both show it under "Needs your attention";
  - the dry-run organiser's log lines in `out.txt` contain no colour codes (`grep -c $'\x1b\['` gives 0).

---

### Stage f1 — Lock documentation *(parallel track; closes #49)*

**Fixes:** O31.
**Model:** Sonnet writer; Opus reviewer, who checks every sentence against `state/mod.rs`.
**Must wait for:** the gap-fix commit, which edits `help/troubleshooting.md` and `help/cli-reference.md`,
and Codex round 2. After that it is independent of every code stage.

**Files:**
- `help/troubleshooting.md` (`:216-249`)
- `help/cli-reference.md` (`:194-224`)
- `docs/api/cli.md`, where it mentions the lock
- `Dev_Notes.md` (`:864`)
- a new documentation-check test in `crates/mm-core/src/state/mod.rs`

**Content:**
- The lock is an operating-system lock on `meedya.lock`. It is released automatically however a run
  ends (normally, by crash or by force-quit).
- **Never delete `meedya.lock`.** On macOS and Linux, deleting it while it is held lets a second copy
  in and move files at the same time.
- `meedya.lock.info` only says who holds it.
- The two real messages, word for word, from `lock_busy_message` and `lock_unavailable_message`.
- **What takes the lock:** `scan --execute`; the macOS app's Execute; `watch --organize` while it is
  moving a folder.
- **What does not:** previews; plain `watch`; the Linux and Windows apps' Execute (#226); Test Mode
  commit and revert (#235).

**Test to write first:** `docs_never_tell_anyone_to_delete_the_lock_file`. It reads every `.md` under
`help/` and `docs/`, and `README.md`, relative to `CARGO_MANIFEST_DIR`, and fails on advice to delete
`meedya.lock`. *(It fails today on `help/troubleshooting.md:243`.)*

**Checks:** the gate, plus `markdownlint` against `.markdownlint.json` if it is installed.

**Hand check:** none on the binary; this stage changes no code. The reviewer compares the documented
messages with the text the lock tests print.

---

### Stage f2 — Organiser and service documentation

**Fixes:** O32, and documents O33.
**Model:** Sonnet writer; an Opus reviewer who checks every claim against the code with a file and line,
as the 2026-09-15 documentation agent did.
**After:** c1 to e3, d3 and f1.

**Files:**
- `help/background-service.md` (the main rewrite)
- `help/getting-started.md`, `help/faq.md` (including removing the false LaunchDaemon claim at `:182`),
  `help/cli-reference.md`, `help/troubleshooting.md`, `help/index.md`, `help/test-mode.md`,
  `help/configuration.md`
- `docs/api/cli.md` (the watch JSON lines, `service status --json`, exit codes)
- `docs/changelog.md` (one entry for c1 to e3 and d3)
- `Dev_Notes.md`, `PROJECT_STATUS.md`, `README.md`
- the context notes: `.claude/CLAUDE.md`, `AGENTS.md`, `.OpenAI/CONTEXT.md` and `.OpenAI/MEMORY.md`

**Keep the "switched off" notices;** stage g removes them. Describe the behaviour as what happens "once
organising is switched back on".

**Must cover:**
- existing files are left alone unless `--organize-existing`; the question it asks and the totals;
- settling: about twice `--settle-secs`, 30 seconds by default, and what it cannot catch;
- the output folder: it must exist, and must be on the same drive unless copy mode is on;
- held back: each reason and what to do;
- "would be renamed again", and how to write a template that settles;
- Test Mode pauses the organiser;
- one organiser at a time;
- Ctrl+C once, or twice;
- `.meedya-partial-…` leftovers after a power cut;
- the service: install starts it and it also starts at login; the log location; `status` and "Needs
  your attention"; Linux lingering; restart after changing settings (settings are read once, at start);
- do not point it at a folder another app manages (Music or iTunes Media, and similar);
- network drives may not report changes (#45);
- **inside a watched folder the organiser's rules win**: renaming a file there by hand gets undone
  (owner decision 4);
- files that arrived while it was stopped are not organised until something in their folder changes
  (owner decision 3);
- Windows: the Task Scheduler route;
- the `~` and `dry_run` settings.

**Test to write first:** extend the documentation-check test. None of these may appear:
`/tmp/meedyamanager`, `set to start at login`, `as they arrive`, `LaunchDaemon`,
`` --settle-secs`, default `2` ``, `organised once now`. *(Fails today.)*

**Hand check:** none on the binary. The reviewer runs the example commands from the new pages with
`--dry-run` against the scratch harness, and confirms the output matches the text.

---

### Stage g — Switch organising back on

**Conditions.** All must be true, and each is recorded in the handoff first:

1. c1, c2, d1, d2, d3, e1, e2, e3, f1 and f2 are committed on the working branch, each with a review loop
   that ended clean, and the number of rounds recorded.
2. **Codex has reviewed the organiser as one body of work** and a round has come back clean.
   - The scope is the organiser code from `6ab0c8d` onwards together with every stage commit: `watch.rs`,
     `scan.rs`, `renamer`, `hold.rs`, `watcher`, `disc` (as it affects the organiser), `service.rs`,
     `service_cmd.rs`, `context.rs`, `config` (the `~` expansion), `organiser_status.rs`, the schema, and
     the help pages.
   - The loop: review, fix, review again, until a round finds nothing real.
   - Run it the way that works on this Mac: `codex exec -s read-only -C <worktree> -c
     model="gpt-6-astra" -o <report> - < <prompt>`. The prompt comes through standard input, so it
     cannot hang waiting for input. Give the prompt this plan's §3 and §4, so it hunts for gaps rather
     than re-finding them.
   - If Codex is unavailable, **do not switch on** with a stand-in reviewer. Wait. Silently handing the
     review to another system is not allowed.
3. **The full gate** on the combined tree, with tests run twice and exit codes read directly.
4. **CI green on Linux, macOS and Windows** for these commits. That means a push, which needs the owner's
   say-so (decision 1). It is the only check that compiles the Linux `systemctl` code, the Windows
   `install_windows`, and `mm-gtk` against the unchanged `RenamePreview` and `RenameSummary`.
5. **The end-to-end dry-run hand check passes on the final head.** It chains every stage's hand-check
   script, on this Mac.
6. **The owner says yes** (decision 1).

**What to change:**
- `ORGANISING_SWITCHED_ON = true`. **Keep the switch**: it is a one-line way to turn organising off
  again if something is found after release.
  - Move the refusal decision into a small function, `switched_off_refusal(switched_on, dry_run)`,
    tested with both values.
  - Replace `watch_organize_without_dry_run_refuses_and_moves_nothing` with those tests, plus a
    `run_until` test showing that a real organiser starts, takes `organiser.lock`, writes
    `organiser-status.json` and stops when asked.
  - Keep to the project's rule: **no test waits for real file events.**
- Change the Windows install message's "switched off" wording. It still refuses, pointing to Task
  Scheduler.
- **Documentation:** remove every "switched off" notice. Find them with
  `git grep -n -i "switched off" -- help docs README.md PROJECT_STATUS.md Dev_Notes.md`; today they
  include `help/background-service.md:7-12`, `help/cli-reference.md:319`, `help/getting-started.md:133`,
  `help/faq.md:28`, `help/index.md:117`, `help/troubleshooting.md:97` and `:347`, `docs/api/cli.md:33`,
  `:91`, `:230` and `:413`, `README.md:231`, `PROJECT_STATUS.md:119` and `:125`, and `Dev_Notes.md:803`.
- Add a changelog entry. Comment on #180 and #231. #231 can close: its last acceptance item is "the same
  protections hold for watch --organize".

**Hand check:**
- **(1)** The dry-run end-to-end script again, on the switched-on build.
- **(2)** Only with the owner's explicit OK: a **real** run confined to the scratchpad, with an empty
  `MM_CONFIG_DIR` and scratch files, never the owner's library.
  - `watch --organize --yes --settle-secs 2`;
  - drop tagged fixtures and see them moved;
  - an `x <Filename>` template is left alone;
  - an arriving rip is held;
  - Ctrl+C twice;
  - then `service status` shows the status file.
- **(3)** Only with the owner's explicit OK, and only if `meedya service status` first says NOT
  INSTALLED: `MM_CONFIG_DIR=<scratch> meedya service install`, then `status`, then `uninstall`. This
  proves `bootstrap` and `bootout` for real. Because `MM_CONFIG_DIR` is kept in the definition, the trial
  service only ever sees scratch settings.

**Commit:** `feat(organiser): switch automatic organising back on (#180)`.

---

## 6c. Dependencies, and what can run side by side

```text
review/round2 + gap-fix commit + Codex round-2 fixes land on the working branch
   │
   ├── f1  (help pages + one test)                  ── side by side with everything below
   ├── d3  (service.rs, service_cmd.rs)             ── side by side with c1 … e1
   │
   └── c1 ─► c2 ─► d1 ─► d2 ─► e1 ─► e2 ─► e3 ─► f2 ─► Codex organiser review ─► g
                                     ▲       ▲
                                     └ needs d3 (install calls the pre-flight)
                                             └ needs d3 (service status)
```

**Why the main line is strictly one after another.** Every stage on it changes `watch.rs`, and most
change `scan.rs`. Two worktrees editing the same functions produce conflicts that no single review has
seen.

**What can safely overlap:**
- **d3** changes only `service.rs` and `service_cmd.rs`; no stage before e2 touches them.
- **f1** changes only help pages and one test in `state/mod.rs`.
- **Optional:** once c1 has landed, the copy-through-a-temporary part of d2 (in `renamer/mod.rs` only)
  could be built beside c2 and d1. The saving is small, and I would not bother.

No stage in c1 to e3 edits `docs/changelog.md` (§6a), so the parallel tracks cannot clash there.

---

## 7. Decisions only the owner can take

Nothing in c1 to f2 waits for these. Where it matters, the recommended answer is what gets built
meanwhile.

1. **Switching organising back on (stage g).**
   - Will you give the final yes yourself, after seeing Codex's verdict?
   - Will you allow the push that lets CI build it on Linux, macOS and Windows?
   - *Recommended: yes to both, at that point.* CI is the only way the Linux service code, the Windows
     code and the Linux app get compiled at all.
2. **Test Mode and the organiser.**
   - The plan makes the organiser **only watch, moving nothing, while Test Mode is on**, because the apps
     already refuse to rename in Test Mode and the organiser runs with nobody watching.
   - A plain `meedya scan --execute` still moves files in Test Mode today.
   - *Recommended: keep the organiser pause as planned; decide `scan` together with #225.*
3. **Files that arrive while the organiser is not running** (overnight, or while the Mac is off) are
   not organised by the service until something in their folder changes. `--organize-existing` does it
   by hand.
   - *Recommended: accept this for now.* A narrower "catch up on what arrived since it last ran" can come
     later. It needs care, because copied files keep their old dates.
4. **Inside a watched folder, the organiser's rules win.** If you rename a file there by hand, or with
   a different template, the organiser moves it back to where its rules say.
   - *Recommended: accept and document.* Respecting hand-made changes needs a record of each file's
     history, which is the library database (#220).
5. **The Linux app's Execute (#226) and Test Mode commit and revert (#235) do not take the write lock.**
   Switch on before they are fixed?
   - *Recommended: yes, and document it.* Harm needs the Linux app and the organiser to pick the same new
     name for two different files within the same fraction of a second. #226 cannot be compiled on this
     Mac. With decision 2, the organiser is paused whenever Test Mode is on.

**Decisions this plan has taken that you might want to reverse** (for information; built as described):
- An incomplete rip holds its folder back, always (the gap fix). This plan extends that to a folder
  with a download still arriving, and to a folder with an unpaired `.bin` — but never to the watched
  folder itself for a `.bin`.
- A manual `scan --execute` still performs a one-off `<Filename>` rename, with a warning. Only the
  organiser refuses.
- 30-second settling.
- No desktop notifications yet: log plus `service status`.
- The organiser never creates a missing output folder, and refuses an output folder on another drive
  unless copy mode is on.

---

## 8. Not now, suggestions, and issues to file

**Issues the orchestrator should file** (checked in the code; none exists yet, by title search):
- **`dry_run: true` in `settings.json5` does nothing, in every command** (O23). Fixed in e2.
- **A `~` in settings paths is taken literally**, and the shipped example uses it. `scan --execute` can
  create a folder named `~` in the current folder (O24). Fixed in e2.
- **macOS `service install` reports success when launchd failed; `service status` says "stopped" when
  nothing is installed** (O12, O13). Fixed in d3. It could also go into #180 as a comment.
- **The organiser picks up files arriving in hidden folders, such as a drive's `.Trashes`** (O19).
  Fixed in e1. It could also go into #180 as a comment.

**Suggestions, not to be built now:**
- The final step of every move could be a "rename only if the name is free" call (`renameat2` with
  no-replace on Linux, `renamex_np` on macOS). It would close the last fraction-of-a-second window for
  every caller, including the Linux app (#226).
- Desktop notifications for held-back folders.
- A catch-up pass in the service for files that arrived while it was stopped (decision 3).
- Automatic clean-up of leftover `.meedya-partial-…` files.
- A rotating log file for the macOS service.
- Removing folders the organiser has emptied.
- JSON Schemas for every command's `--json` output. Only the status file gets one in this plan.
- Re-reading settings while running.
- MeedyaSuite-core already has a cue sheet parser (`meedya-library-import/src/cuesheet.rs`). It is worth
  comparing with this project's `disc/cue.rs`, under the "shared code belongs upstream" rule, as part of
  #217, not this plan.

**Already out of scope (§15's "Not now", still true):**
- whole-folder moves (#217) and fingerprinting;
- polling for network drives (#45);
- stopping gracefully on the service's stop signal;
- running in the background on Windows, beyond the Task Scheduler route;
- raising `rust-version` (#234);
- the engine bridge's missing disc protection (#223);
- the numbering bug (#224). The organiser keeps forcing "skip".

---

## 9. What I could not verify

- **The gap-fix commit's final form.** I read work in progress. If its names or behaviour change before
  it lands, c2 must follow what actually landed.
- **Whether older Mac drives (Mac OS Extended, not APFS) cause a rename loop** by storing accented names
  differently. Not tested; I have no such drive. c1's move history guards against it either way.
- **Linux behaviour** of `systemctl --user is-active` for a unit that does not exist. I believe it
  prints "inactive", so today's "not found" check never matches. That is from reading, not running.
  d3's "no unit file means not installed" rule avoids depending on it.
- **That launchd accepts the same file for both log streams.** It is widely used; `plutil -lint` in d3
  checks the file's form, not launchd's behaviour. Stage g hand check (3) proves it for real.
- **Anything on Windows or in `mm-gtk`.** Neither can be built on this Mac; CI is a condition of g.
- **Finding #18 in full**, and the C# test point. Not organiser code, and not re-checked.
- **Whether the organiser's per-folder `scan` is fast enough on a very large moved-in folder** (tens of
  thousands of files). Not measured. d2's stop checks keep it stoppable either way.

---

# Stage e4 — automatic catch-up (added 2026-09-25)

## Owner decisions that override parts of the e4 plan below (2026-09-25, final)

1. **What gets organised when something arrives becomes a user setting.**
   - **The default: the whole folder is organised, including files that were already there.**
     The other choice is "only the files that arrived".
   - This applies to the running organiser and to catch-up alike.
   - Build it as a setting in `settings.json5`, for example
     `watch.organise_on_arrival: "whole_folder" | "new_files_only"`, with `whole_folder` as the
     default. Add it to the JSON Schema, the help pages, and `meedya config show`.
   - Place: in e1, alongside "settled" and "arrival", or in e4. The orchestrator decides at build
     time.
   - **Whichever setting is chosen, the organiser must never treat its own moves as arrivals.**
     Today a file it moves into a folder raises an event there, and that folder is then organised.
     This was found by the e4 planner, and fixing it is added to e1.
2. **Catch replaced and edited files too.**
   - The planner's recommendation was to accept missing them. The owner chose to catch them.
   - So at each start, catch-up also compares every recorded file's size and modification or
     change times, not only file names. A file whose details differ is treated as arrived.
   - This costs roughly 10–13 s per start for about 150,000 files on this Mac, measured, instead
     of about 0.3 s. **Raise the start budget accordingly.**
   - Catch-up must also run **in the background after start-up**, so the service starts at once and
     the file check runs while events are already being watched. That keeps the slow part off the
     start-up path.
   - Keep everything else in the plan: the record, the first-run starting point, and the rules
     that stop anything being lost.

## Found by the e4 planner, to be added to stage e1 (which is not built yet)

1. The organiser's own moves trigger organising in the destination folders; see decision 1 above.
2. Windows and Linux system folders are not skipped: `$RECYCLE.BIN`, `System Volume Information`
   and `lost+found`. These are the equivalents of `.Trashes`, which O19 already covers.
3. d1's `--organize-existing` sweep does not wait for files to settle, so a file still being copied
   in at start-up could be organised half-written.

## The planner's e4 plan, as written

<!-- (C) 2025-2026 MWBM Partners Ltd -->
<!-- Planning output for session 07a21012, written 2026-09-25 by an Opus planning agent.
     <repository> is the MeedyaManager repository root; <session scratchpad> is the session's temp folder. -->

# Organiser fix plan — new stage e4: automatic catch-up

**Written:** 2026-09-25, by the planning agent (Opus 5.5), for the orchestrator of session `07a21012`.

**Changed nothing** in any repository or on GitHub. This is the only file written.
- The scratch files made for the live checks in §2 were deleted.
- The three test disk images were detached and deleted.

**Slots into** `.claude/organiser-fix-plan.md`.
- It uses that plan's rules (§6a) and its hand-check harness (§6b).
- It is in the same format as the other stages, so §5 can be pasted in as a new section after stage e3.

**Read against:** the local branch `org/c1-rename-loop` at `bc69741`. That is stage c1, built on `org/line`, which already holds stage d3.
- Stages c2 to e3 are **not built yet**.
- Where this stage hooks into something they add, it uses the name their plans give. The builder must use whatever name actually landed.

---

## 1. The stage on one page

- **What it does.**
  - When `meedya watch --organize` starts, or the background service that runs it, it works out which files arrived in the watched folders while it was not running.
  - It then organises the folders those files arrived in.
  - It does this through exactly the path the running organiser uses. So every protection the other stages add applies: settling, held-back rips, the rename-loop guards, the Test Mode pause, the write lock and the output-folder checks.
- **The rule it follows.**
  - *Catch-up organises a folder only if the running organiser would have organised it, had it been running when the file arrived.*
  - It never organises a folder merely because the folder exists.
- **How it tells "arrived while stopped" from "already there".**
  - It keeps a **record** for each watched folder. The record lists:
    - every folder beneath the watched folder;
    - each folder's own two timestamps;
    - a **fingerprint** of every file name in each folder. A fingerprint is a short number worked out from the name; the same name always gives the same number.
  - At start, it reads each folder's own details. That is cheap.
  - It lists the contents only of folders whose details have changed.
  - A file name that is not in the record has arrived.
  - **File dates are never trusted.**
- **The first run ever.** There is no record yet. So it records what is there as the starting point, organises nothing, and says so.
- **Cost.** Measured on this Mac for 150,000 files in 16,251 folders (§2):
  - reading every folder's details took about 0.3 seconds;
  - listing every folder took about 10 to 13 seconds.
  - So a normal start stays well under 2 seconds.
  - A first run, or a drive formatted FAT or exFAT, lists every folder. That takes about 10 to 20 seconds on this Mac. It runs in the background in short steps, and Ctrl+C still works.
- **Where it goes.** After e3 and before f2: `… → e2 → e3 → e4 → f2 → Codex organiser review → g`. The reasons are in §4.
- **Model.** Opus to build (§5, "Model").
- **Owner decisions.** Two, each with a recommended answer that gets built meanwhile (§7).

---

## 2. What I checked, and how

- **Read in full:**
  - `.claude/organiser-fix-plan.md`: stages c1 to g, §6a, §6b and §7;
  - `.claude/HANDOFF.md` §0, including the owner decisions of 2026-09-25.
- **Read the code at `org/c1-rename-loop` (`bc69741`):**
  - `watch.rs`: the whole organiser, including `MoveHistory` and `sweep_roots`;
  - `scan.rs`: `run_with`, `ScanHooks`, `ScanOutcome` and the lock handling;
  - `watcher/mod.rs`: the download-in-progress rules, `should_ignore` and `scan_directory_below`;
  - `state/mod.rs` (where the lock lives), `config/mod.rs` (`app_config_dir`), `test_mode.rs`, and the service subcommands in `service_cmd.rs`.
- **What that reading established:**
  - Every organiser file lives in `app_config_dir()`. That is `MM_CONFIG_DIR` if it is set, otherwise the platform's settings folder.
    - `meedya.lock` is there (`state/mod.rs:481-487`).
    - e3 puts `organiser.lock` and `organiser-status.json` there too.
  - `test_mode::is_enabled()` re-reads the Test Mode manifest on every call (`test_mode.rs:428`). So e2's pause lifts as soon as Test Mode is switched off, without a restart.
  - The workspace setting `unsafe_code = "warn"` (`Cargo.toml:185`) flags low-level code.
    - `mm-core` already makes one low-level `libc` call, with its own local permission (`service.rs:618`, `geteuid`).
    - `libc` is already a `mm-core` dependency on macOS and Linux.
    - So the one "which kind of drive is this?" call this stage needs adds no new dependency.
- **MeedyaSuite-core, checked first:** the local checkout at `aad49c7` and the pinned `222ca75`.
  - Nothing there records "what has been seen": no watcher, no scan state, no file index.
  - The only folder walk is a tagging helper (`meedya-metadata/src/writer.rs:193`).
  - Nothing in this repository does it either. I checked `state`, `integrity` and `health`.
- **`notify` 7 cannot replay history.** Its macOS back end always asks the system for "events from now on" (`notify-7.0.0/src/fsevent.rs:274`, `kFSEventStreamEventIdSinceNow`), and does not let a caller change that.
- **Live checks on this Mac.** All ran in the session scratchpad, and nothing was left behind.
  1. **APFS, the Mac's own disk format:**
     - `cp -p` and `ditto` copies keep the original **modified** and **created** dates.
     - Only the file's "change time" is set to now. That is Unix's `ctime`: when anything about the file's entry last changed.
     - **Moving** a file sets its change time to now.
     - **Writing an extended attribute** onto an old file **also** sets its change time to now. An extended attribute is a hidden label, which macOS adds for many reasons.
     - Adding a file changes its folder's modified time. Writing into a file that is already there does not.
  2. **Cost, for 150,000 empty files in 16,251 folders.** Measured with a Python script. The computer had just read those folders and still remembered them (a "warm" cache).

     | What was read | Time (two runs) |
     | --- | --- |
     | Every folder's contents (names only) | 9.6 s and 13.2 s |
     | Every folder's contents, plus every file's details | 18.1 s and 15.5 s |
     | Only each folder's own details | 0.26 s and 0.34 s |

     - **The C program `find`** gave the same pattern on 2,201 folders: 2.8 s. That is about **1.3 ms to list one folder**, against **6 to 9 µs to read one folder's details**. So Python was not the cause.
     - **An "endpoint security" extension is active** on this Mac: NordVPN's Shield threat protection.
       - Tools like that inspect every folder a program opens. That is very probably why listing is this slow here. **Not proven.**
       - Many real Macs run something similar, so the design must not depend on listing being cheap.
     - **Not measured:** timings straight after a restart (a "cold" cache).
  3. **FAT32 and exFAT disk images**, mounted with the Mac's own drivers. FAT32 and exFAT are the formats of most USB sticks and camera cards.
     - Folder times **did** change when a file was added or moved in.
     - The change time is always a copy of the modified time. Moving a file did **not** change it.
     - FAT32 times go in **2-second steps**.
     - File identity numbers stayed the same across a move and an unmount.
  4. **An HFS+ disk image** (Mac OS Extended, the older Mac format):
     - folder times are whole seconds;
     - **two files added within the same second left the folder's times exactly the same.**
- **The fingerprint test values** in §5 (test 1) were computed with an independent script.

---

## 3. The hard parts, decided

### 3a. How to tell "arrived while stopped" from "already there"

Each candidate is judged on correctness first, then on cost.
- **"Dangerous"** means it could make an old file look new. That would move a file the owner said to leave alone.
- **"Safe"** means it could only miss an arrival. A miss is today's behaviour.

| Signal | What it cannot detect, or gets wrong | Direction of its mistakes | Verdict |
| --- | --- | --- | --- |
| **A file's modified or created date**, compared with when the organiser stopped | Copies keep both dates (checked with `cp -p` and `ditto`). Downloads can set old dates. A clock change moves the point being compared against | Both | **Rejected.** The owner ruled it out, and the live check confirms why |
| **A file's change time** (`ctime`), compared with the last check | A hidden label written onto an old file bumps it (checked on APFS); so do permission changes and tag edits. On FAT and exFAT it is only a copy of the modified time, and moving a file does not bump it (checked). Windows' standard library does not offer it. A clock set back makes new files look old; a clock that ran fast and was corrected makes old files look new. On a network drive it comes from the server's clock | **Dangerous** as well as safe | **Rejected** as the signal. Kept only as a second shortcut timestamp on folders (below), where a false "changed" costs just one extra listing |
| **File identity numbers** (Unix "inode" numbers: the number a disk uses for a file) | Some drivers make them up. Linux's FAT driver hands out a fresh number for an entry it has not cached, so the same file can get a new number at the next mount (from the kernel source; not run here). Numbers are reused after a deletion, so a new file can inherit an old one's number. Network drives may make them up. The Mac's own FAT driver kept them stable in the check, so this is about other systems | **Dangerous** on those drives | **Rejected** for the record. Stage c1 still uses them within one run, where they are safe |
| **A folder's modified time**, plus its change time on macOS and Linux | Cannot see a file edited in place: that does not touch the folder. HFS+ and FAT times are coarse, so two changes can leave identical times (checked on HFS+); Linux also stamps times coarsely. Other systems writing to a FAT or exFAT drive may not update folder times at all (not checked). A network drive may briefly show old times from its cache | **Safe only.** Used only to *skip listing* a folder; the names make the decision | **Used, as a shortcut only.** Three guards: (1) always list every folder on FAT, exFAT, Windows or an unknown file system; (2) list next time any folder whose times were within 5 seconds of when they were recorded (the "recheck" rule, for coarse clocks); (3) compare times only for being *different*, never earlier or later, so clock changes do not matter |
| **A record of the file names in each folder** | Cannot see a file **replaced by another of the same name**, or **edited in place**. A folder renamed or moved within the watched folder looks new, so it is organised, as the running organiser would do. If names change form wholesale (a library copied to a drive that stores accented letters differently, or a different drive mounted under the same name), all of them look new; the "looks very different" rule (§3d) catches that. Needs a first run to make a starting point | Safe for replacements and edits. Dangerous only for wholesale name changes, which the guard catches | **Chosen as the deciding signal** |
| **macOS's own change history** (FSEvents can replay changes since a saved point, even across restarts) | Only macOS. `notify` cannot use it. It needs low-level calls into the operating system (new `unsafe` code, which CI's Linux runs cannot test). It misses changes made to an external drive while it was plugged into another computer. Its history can be lost or reset | Safe | **Rejected for now.** Suggested for later as a speed-up on top of the record (§8) |

**Why names, and not "names plus each file's size and date".**
- Reading each file's size and date means one detail-read per file, not per folder.
- In the measurement that was 18 s against 13 s, and it would be far worse on a network drive.
- What it would add is catching a same-name replacement or an in-place edit. Neither is an arrival.

**So "arrived" means precisely:** *a file name that has appeared in a folder since that folder was last dealt with.*

These count as arrivals:
- a new file;
- a file copied or moved in;
- a file renamed in place;
- a whole folder moved in or created.

These are **not** arrivals:
- a file that disappeared;
- a file edited in place, or replaced under the same name;
- hidden files and system clutter (§5 step 1);
- anything still downloading (§5 step 1).

**What happens in the situations the brief names, and a few others:**

| Situation | What catch-up does |
| --- | --- |
| **The clock is changed** (either way, or a time-zone change) | Nothing different. Times are only compared for being different, never for order, and names decide. At most, the recheck rule lists a few extra folders |
| **The library is restored from a backup to the same place, with the same names** | Nothing arrives, because the names match. Folders whose times changed are listed once and updated |
| **The library is restored under different names, or a different drive is mounted under the same name** | "Looks very different" (§3d): nothing is organised, a new starting point is recorded, and the user is told loudly |
| **The settings folder, and so the record, is restored from an older backup** | Everything that arrived since that backup looks new, and its folders are organised again. Files already organised do not move again (c1 sees they are in place). Files deliberately left alone since then (for example after `reset-catch-up`) would be organised |
| **A FAT or exFAT drive** | Every folder is listed at every start. Names decide, so the answer is correct; it is just slower |
| **A network drive** | Folder times are trusted, as best effort. Files that other computers added are caught at the next start, which the running watcher cannot do at all (#45) |
| **Files renamed or moved within the watched folder while it was stopped** | The new name is an arrival, so its folder is organised, as the running organiser would have done. A renamed folder is a new folder, and is organised whole |
| **A file half-copied when the organiser stopped** | Its folder was still queued, so it is marked `owed` (§3b), and it is caught up. It goes through e1's settle check first |
| **A download still running at start** | Its names count as "still arriving", so it is not an arrival until it finishes |
| **The Mac was off for a week** | The same as overnight. The cost does not depend on how long it was stopped |
| **A watched folder is removed from settings, then added back later** | Its record file is kept, so catch-up carries on from where it was |
| **A watched folder that was inside another watched folder is removed from settings** | Its folders are already known to the outer folder's record by name. They are recorded as a starting point, never organised (§3b, rule I1) |

### 3b. The record

**Where it lives.** In the settings folder (`app_config_dir()`), next to `meedya.lock`, `organiser.lock` and `organiser-status.json`.
- Its path is `organiser-catchup/<key>.json`: one file per watched folder.
- `<key>` is the fingerprint (§5 step 2) of the watched folder's whole real path, written as 16 hexadecimal digits (digits 0 to 9 and letters a to f).
- The file also states that path. A file whose stated path does not match is treated as missing, and is overwritten at the next save.

**Why the settings folder:**
- nothing is ever written into the user's media folders;
- a throwaway `MM_CONFIG_DIR` isolates it automatically, for tests and hand checks;
- the service keeps the same `MM_CONFIG_DIR` as the terminal that installed it (stage d3), so both use one record.

**Why one file per watched folder:** a terminal run given some other folder must never disturb the service's record for the usual folders.

**Format.** JSON, with a schema at `config/schemas/organiser-catchup.schema.json`:

```json
{
  "format_version": 1,
  "root": "/Users/example/Media/Incoming",
  "written_by": "meedya 1.4.0-alpha.1",
  "starting_point_at": "2026-09-25T22:14:03+01:00",
  "saved_at": "2026-09-26T07:58:12+01:00",
  "name_fingerprint": "fnv1a-64",
  "folders": {
    "": {
      "modified_ns": 1790354516000000000, "changed_ns": 1790354516000000000,
      "recheck": false, "owed": null,
      "names": "0cdd348313fb87edaf63dc4c8601ec8c", "subfolders": "…"
    },
    "Artist/Album": {
      "modified_ns": 1790354517123456789, "changed_ns": 1790354517123456789,
      "recheck": false, "owed": null, "names": "…", "subfolders": ""
    },
    "Arrived/New": {
      "modified_ns": null, "changed_ns": null,
      "recheck": true, "owed": "subtree", "names": "", "subfolders": ""
    }
  }
}
```

**What each part means:**
- **`folders`** is keyed by the path **relative to the watched folder**, with `/` between the parts on every system. `""` is the watched folder itself.
  - A path that is not valid text (possible on macOS and Linux) is stored as `hex:` followed by its raw bytes in hexadecimal. That way it survives a save and reload exactly.
  - That matters: a folder name that came back different would look like a new folder, and would be organised.
- **`modified_ns` and `changed_ns`** are the folder's **own** times, in nanoseconds since 1970.
  - `changed_ns` is `null` on Windows.
  - `null` means "unknown: list this folder next time".
- **`names`** holds the sorted fingerprints of the file names directly in that folder, 16 hex digits each, run together.
- **`subfolders`** holds the same for the names of its subfolders. That includes subfolders that are skipped or belong to another watched folder. See rule I1 for why.
- **`owed`** is `null`, `"folder"` or `"subtree"`. It means "work is still owed here: queue it at the next start, whatever else is true".
- **`recheck: true`** means "list this folder next time, even if its times look unchanged" (§3a, the coarse-clock guard).

**Size, for 150,000 files in about 16,000 folders:**
- about 170 bytes per folder and 16 bytes per file: roughly **2.7 MB + 2.4 MB ≈ 5 MB** on disk, and about the same in memory;
- storing the names in full would be about 8 to 9 MB;
- base64, a denser way of writing bytes as text, would save about 0.6 MB, but needs a new direct dependency. Not worth it now.

**How it stays small:**
- fingerprints, not names;
- folders stored relative to the watched folder;
- nothing stored per file except its fingerprint;
- written only when something has changed, and at most once a minute (§5 step 4.7).

**A hard limit.** A watched folder with more than 1,000,000 files or 200,000 folders has catch-up switched off, with a message saying why.
- At that size the record would pass 35 MB, and the check could not meet its budget.
- The running organiser is unaffected.

**The rules the record must always keep.** The tests and the reviewers check against these.

- **I1. Every folder the organiser knows about has an entry.** Each entry lists its subfolders' names as well as its files.
  - A folder counts as **new** only if its name is missing from its parent's recorded `subfolders`.
  - Some folders are named there but have no entry of their own. That happens when a folder used to belong to another watched folder, when recursion was off, or when a skip rule changed. Such a folder is recorded as a starting point and is **never** caught up.
  - A newly found folder gets an entry **at once**. If it has not been dealt with yet, the entry has times `null` and `owed` set.
- **I2. An entry's times were always read before its names were listed.** So anything that changes the folder afterwards shows up as different times next time.
- **I3. An entry's names hold only names that have been dealt with.** A name is dealt with if it was:
  - present at the starting point; or
  - present when the folder was organised; or
  - present when the folder was checked and nothing had arrived.

  The record never holds a name that was still arriving, or a name that appeared after the folder was last dealt with.
- **I4. A folder with work still owed has `owed` set.** It is queued at every start until it has been dealt with.
- **I5. Only two things write the record:** a real organiser holding `organiser.lock`, or `service reset-catch-up` holding it. **A preview never does.**
- **I6. Every save replaces the whole file at once.** A temporary file is written first, then swapped in. So a reader never sees half a file, and a crash leaves the previous version whole.

**Why a crash, a kill or a power cut is safe.**
- Because of I2 and I3, a record that is out of date only ever holds *older* names.
- So anything that arrived since that save still shows up as new at the next start.
- The cost of an out-of-date record is re-organising some folders that were already dealt with. Since stage c1, that moves nothing.
- **The one gap:** a file that arrived *while the very first starting point was being recorded*, if the organiser then dies within the following minute.
  - Its name may be recorded without `owed`, so it waits for the next change in its folder.
  - §5 step 4.7 narrows that window as far as it can go.
- **A service stop is the same as a kill.**
  - d2 decided not to handle the service's stop signal. So a service stop (logout or shutdown) ends with no final save.
  - The last save stands. It is at most a minute old, and by the argument above that is safe.

### 3c. The first run ever

- **What it does.** With no record for a watched folder, it lists every folder beneath it, records the result as the starting point, and queues nothing.
  - That is "existing files are left alone", exactly as stage d1 promises.
  - Names still arriving are left out, so they count as arrivals once they have finished.
  - Some folders are queued by the running organiser meanwhile, because files arrived during the check. They are marked `owed` when the record is saved.
- **How the user is told:**
  - the start-up line;
  - the log;
  - `meedya service status`;
  - and d1's start question, which on a first run says the files already there will be recorded and left alone.
- **A watched folder added later** gets a first run of its own. The other folders' records are untouched.
- **Under `--dry-run`** nothing is written. The preview says what a real run would record.
- **Under `--organize-existing`, when the sweep is accepted** (stage d1): the sweep organises everything, so catch-up adds nothing. The record is built as the sweep deals with each folder.

### 3d. Safety

Catch-up only ever **decides which folders to put in the queue**. It never plans or makes a move itself. Everything after that is the running organiser's own path.

| Protection | How catch-up gets it |
| --- | --- |
| **Never move a file that is still arriving** | Four layers. (1) Names that look like a download in progress never count as arrivals, and are never recorded. (2) Every caught-up folder goes through e1's settle queue: two identical listings a settle period apart, with no download in progress. (3) c2's hold rules apply when `scan` runs. (4) Moves are planned by `scan` after settling, never from the check's own listing |
| **Held-back rips (c2)** | Through the per-folder function. A held folder stays `owed`, so it is looked at again at every start until it is complete, and reported each time, as c2 wants |
| **Rename-loop guards (c1)** | Through the per-folder function. `MoveHistory` is shared with the rest of the run. A file left alone ("would be renamed again", a conflict, a failed move) counts as dealt with, so it is **not** retried at every start |
| **Settling (e1)** | Every catch-up folder is put in e1's queue. None is ever organised directly |
| **Test Mode** (e2; also the owner's decision of 2026-09-25 that Test Mode refuses renames) | e2 pauses the organiser before each folder while Test Mode is on. Caught-up folders stay queued and `owed`, so they survive restarts until Test Mode is switched off. If #225 lands, `scan` refuses as well: a second barrier |
| **The write lock** | Moves go through `scan`, which takes `meedya.lock`. A busy lock uses e3's waits, and the folder stays `owed` |
| **The output folder (e2)** | e2 checks before starting and before each folder. If the output folder is missing, everything is held and the folders stay `owed` |
| **Real paths (e2)** | The record is keyed by the real path. Without that, `/tmp/x` and `/private/tmp/x` would get two records, and events would not match |
| **One organiser at a time (e3)** | `organiser.lock` makes the organiser the record's only writer |
| **Ctrl+C (d2)** | The check tests the stop flag between folders. A stopped check leaves the folders it has not reached exactly as recorded. The final save happens before "Watcher stopped" |
| **A folder that looks very different** | If at least 200 files were recorded for a watched folder and more than half of them are no longer there, catch-up **organises nothing** for it. It records what is there now as the new starting point, and says so loudly (log, status, "Needs your attention"). This catches a restored or moved library, a different drive mounted under the same name, and names changing form wholesale |

### 3e. Scale: the budget, and how it is met

**Every start after the first**, on APFS, HFS+, ext4, XFS, Btrfs, ZFS and network drives:
- it reads each recorded folder's own details once;
- it lists only the folders whose details changed, which in practice means the folders where something arrived;
- it reads **no file's details or tags during the check**.
- **Budget: at most 2 seconds** for 150,000 files in 16,000 folders on this Mac's internal disk, with up to 100 changed folders.
  - The basis, measured: about 0.3 s of detail reads, plus about 1.3 ms for each changed folder.
  - The hand check measures the real program (§5, hand check (f)).

**The first run; FAT and exFAT drives; Windows; unknown file systems:**
- one listing per folder: about 10 to 20 seconds per 16,000 folders on this Mac;
- once per watched folder for a first run;
- at every start for those drives, which usually hold far fewer files.

**It never blocks the organiser:**
- The check runs in steps of at most **250 ms**, one step per pass of the event loop. Between steps, the loop waits for file events and handles them.
- So when nothing else is happening, the check uses at most about half of the disk's time. Live events keep being queued, and Ctrl+C is honoured within about half a second.
- **Folders under a watched folder that is still being checked are queued, but not organised, until its check finishes.**
  - This stops the check and the organiser updating the same entries at the same time.
  - The settle wait (about a minute by default) is longer than a normal check anyway.

**Tags are read only in folders being organised.** So that cost follows what arrived, never the size of the library.

**What cannot be promised:** speeds straight after a restart, on spinning disks, and on slow networks.
- The check's work is one small read per folder.
- How long that takes there was not measured.
- It is shown in the start-up line and in `service status`, so it can be seen.

### 3f. What the user sees

| Where | What |
| --- | --- |
| **The start question** (d1's, on a terminal without `--yes`) | One sentence added: *"Files that arrived since it last ran are organised too; files that were already there before are left alone — add --organize-existing to organise them too."* On a first run: *"This is the first run for <folder>: what is there now will be recorded and left alone."* |
| **A check that takes more than 1 second** | *"Catch-up: checking <folder>…"*, so a long first run or a slow drive is not silent |
| **Start-up line, normal** | *"Catch-up: 12 files arrived in 3 folders while MeedyaManager was not running (record saved 22:14 on 25 Sep). They will be organised once nothing in those folders has changed for about a minute. (Checked 16,251 folders in 0.4 s; listed 14.)"* |
| **Nothing arrived** | *"Catch-up: nothing arrived while MeedyaManager was not running. (Checked 16,251 folders in 0.3 s.)"* |
| **First run** | *"Catch-up: this is the first run for <folder>, so there is nothing to catch up on. What is there now (150,000 files in 16,251 folders) has been recorded as the starting point and left alone. From now on, files that arrive while MeedyaManager is not running will be organised when it next starts."* |
| **Looks very different** | *"Catch-up skipped for <folder>: it looks very different from last time — 9,300 of the 15,000 files recorded then are no longer there. This happens after restoring a backup, moving the library, or connecting a different drive under the same name. Nothing will be organised from it. What is there now has been recorded as the new starting point. If some of those files do need organising, preview it first with: meedya scan "<folder>""* |
| **Record unreadable** | *"Catch-up skipped for <folder>: its record could not be read (<reason>). It has been kept as <file>.unreadable. What is there now has been recorded as the new starting point; nothing was organised from it."* |
| **Too big** | *"Catch-up is switched off for <folder>: it holds more than 1,000,000 files (or 200,000 folders), too many to check quickly at every start. Files that arrive while MeedyaManager is running are still organised."* |
| **Under `--dry-run`** | Each line above says what a real run *would* record, and records nothing |
| **A drive listed in full** | Added to the line: *"(<folder> is on a FAT or exFAT drive, so every folder is listed at each start; this takes longer.)"* Or *"on Windows"*, or *"on a file system MeedyaManager does not recognise"* |
| **Test Mode on** | e2's "only watching" message, plus: *"…the 12 files that arrived will be organised once Test Mode is switched off."* |
| **`watch --json`** | One compact line per catch-up result, like c1's other lines: `{"time","event":"catch_up","folder","state","arrived_files","arrived_folders","folders_checked","folders_listed","seconds","message"}`. `state` is one of `checking`, `done`, `starting_point_recorded`, `skipped_looked_different`, `skipped_record_unreadable` or `switched_off_too_big` |
| **The log** | The same human lines. For the service that is its log file (stage d3), without colour codes (stage e3) |
| **`organiser-status.json`** (e3) | See the list below this table |
| **`meedya service status`** | *"Catch-up (files that arrived while MeedyaManager was not running): /…/Incoming — checked 07:58 (16,251 folders in 0.4 s): 12 files had arrived in 3 folders; 1 folder is still waiting (held back)."* Attention items appear under e3's "Needs your attention" |

**What `organiser-status.json` gains:**
- A `catch_up` list, with one entry per watched folder. Its fields are:
  - `folder`, `state`, `record_saved_at`, `checked_at`, `check_seconds`;
  - `folders_checked`, `folders_listed`;
  - `lists_every_folder`: null, `"first_run"`, `"fat_or_exfat"`, `"windows"` or `"unknown_file_system"`;
  - `arrived_files`, `arrived_folders`, `folders_waiting`.
- New "Needs your attention" kinds: `catch_up_skipped`, `catch_up_record_unreadable`, `catch_up_record_not_saved`, `catch_up_switched_off` and `catch_up_folder_unreadable`.

---

## 4. Where it goes, and why

**After e3 and before f2:** `c1 → c2 → d1 → d2 → e1 → e2 → e3 → e4 → f2 → Codex organiser review → g`.

- **Not before e1.** Catch-up needs three things from e1:
  - its folder-level settle queue, which is how catch-up avoids moving a file still arriving;
  - its hidden-folder rules;
  - its handling of folders moved in whole. A caught-up new folder is a subtree, exactly like one moved in while running.
- **Not before e2.** It needs:
  - real paths, because the record is keyed by them. Without them, the `/tmp` and `/private/tmp` mismatch (O21) would give two records;
  - the Test Mode pause;
  - the output-folder checks.
- **Not before e3, and not beside it.**
  - Only e3's `organiser.lock` stops two organisers writing one record at the same time.
  - Catch-up reports through e3's status file and "Needs your attention", and it uses e3's lock waits.
  - Building it beside e3 would mean two worktrees editing `watch.rs` and the status file at once. §6c forbids that, for good reason.
- **Not folded into e1.** e1 is already the plan's one fiddly, Opus-built stage. Catch-up is the second, and deserves its own review.
- **Before f2**, so f2 documents the finished behaviour once.
- **Before the Codex review and g**, so both cover it.

---

## 5. Stage e4 — Catch up on files that arrived while the organiser was not running

**Fixes:** O34, a new label.
- *Files that arrive while the organiser is not running (overnight, while the Mac is off, while the service is stopped or has crashed) are never organised until something else changes in their folder.*
- This is owner decision 5 of 2026-09-25: build automatic catch-up now.
- It supersedes §7 decision 3.
- It gives back what the old start-up sweep promised ("install the service and forget about it"), without sweeping the whole library.

**Model:** **Opus** to build.
- This is the plan's second genuinely fiddly piece, after e1.
- Three things decide whether catch-up loses work or, worse, moves files the owner said to leave alone: what counts as "dealt with", when the record is saved, and what a crash leaves behind.
- **A cheaper split, if the orchestrator prefers:**
  - steps 1 to 3 (the name rules and the pure `catch_up.rs` module, with their tests) are specified precisely enough for **Sonnet**;
  - steps 4 to 7 (wiring into the organiser, saving the record, the status and the reset command) must be **Opus**.
- **Review:** a **fresh** Opus agent reviews it, and the handoff records that the reviewer is the same model.
- **The Codex organiser review** is given rules I1 to I6 (§3b) and asked to try to break them.

**Needs:** c1, c2, d1, d2, e1, e2 and e3 landed.

**Issue:** file one before work starts, as the project rule requires. For example: *"Organiser: catch up on files that arrived while it was not running (owner decision 5, 2026-09-25)"*, linked from #180.

**Files:**
- **new** `crates/mm-core/src/catch_up.rs` (and `pub mod catch_up;` in `crates/mm-core/src/lib.rs`)
- `crates/mm-core/src/watcher/mod.rs` (shared name rules)
- `crates/mm-cli/src/commands/watch.rs`
- `crates/mm-cli/src/commands/organiser_status.rs` (from e3)
- `crates/mm-cli/src/commands/service_cmd.rs`
- **new** `config/schemas/organiser-catchup.schema.json`
- `config/schemas/organiser-status.schema.json` (from e3)
- No `Cargo.toml` change: `libc` is already a `mm-core` dependency on macOS and Linux.

**What to build:**

1. **Shared name rules, in `watcher/mod.rs`. One list each, never two.**
   - **`pub fn is_arriving_by_name(name: &OsStr, siblings: &HashSet<OsString>, inside_aria2_download: bool) -> bool`**
     - It covers:
       - the endings in `DOWNLOAD_IN_PROGRESS_EXTENSIONS`;
       - a sibling named `<name>.aria2`;
       - aria2's whole-folder download. The walker works this out from the parent folder's listing, as `scan_directory_below` already does;
       - rsync's temporary names;
       - Office's `~$` files.
     - It answers from the folder listing alone, so the check never reads a file's details.
     - `is_download_in_progress_impl` must call it for those parts. It keeps its own existence checks only for callers that have no listing.
   - **`pub fn is_clutter_name(name: &OsStr) -> bool`**
     - It covers:
       - any name starting with `.`. That includes `.DS_Store`, and the `._name` files macOS writes on FAT, exFAT and network drives;
       - `Thumbs.db`, `ehthumbs.db` and `desktop.ini`, in any letter case;
       - macOS's custom-icon file `Icon\r`;
       - Test Mode copies (`test_mode::is_test_mode_copy`);
       - `~$…`;
       - a trailing `~`.
     - These appear when somebody merely *looks at* a folder (in Finder or Explorer). So they must never count as arrivals.
   - **`pub fn is_skipped_folder_name(name: &OsStr) -> bool`**
     - It covers:
       - a leading `.`;
       - a name ending `.download`;
       - `$RECYCLE.BIN`;
       - `System Volume Information`;
       - `lost+found`.
     - **If d1 and e1 put an equivalent rule in `watch.rs`, move it here and call it from every place.** The sweep, the events and catch-up must share one set of folder rules.
     - The three new names are the Windows and Linux twins of O19 (`.Trashes`). A drive formatted on Windows keeps deleted files in `$RECYCLE.BIN`.

2. **The record and the check, in the new `crates/mm-core/src/catch_up.rs`.**
   It is pure: the file system sits behind a small trait (an interface), so tests can count every read.

   - **`trait FolderReader`** has three methods:
     - `fn details(&mut self, dir: &Path) -> io::Result<FolderDetails>`: the modified time, the change time (macOS and Linux only) and the device number;
     - `fn list(&mut self, dir: &Path) -> io::Result<Vec<ListedEntry>>`: each entry's name, and whether it is a folder, a file or a link;
     - `fn trust(&mut self, dir: &Path, device: u64) -> TimeTrust`.
   - **The real reader:**
     - uses `symlink_metadata` for details;
     - uses `read_dir` with `DirEntry::file_type()` for listing. Never `path.is_dir()`: that reads every entry's details and follows links;
     - never follows a linked folder.
   - **`pub fn name_fingerprint(name: &OsStr) -> u64`**, using FNV-1a (64-bit), a published and very simple way of making such fingerprints.
     - On macOS and Linux it works over the name's raw bytes. On Windows it works over the name's UTF-16 units, each taken as two bytes, low byte first.
     - It needs no dependency.
     - It gives the same answer on every machine and every Rust version. Rust's built-in hasher does not promise that.
     - **If this function ever changed, every name would look new.** Test 1 pins it.
   - **`TimeTrust`** says whether a drive's folder times can be relied on:
     - **`Trusted`:** `apfs`, `hfs`, ext2/3/4, XFS, Btrfs, ZFS, F2FS and `tmpfs`. Also network drives (`smbfs`, `nfs`, `afpfs`, `cifs`, SMB2), as best effort: their folder times come from the server.
     - **`ListEveryFolder`:**
       - `msdos`, `vfat` and `exfat`;
       - anything reached through FUSE. On Linux that includes `ntfs-3g` and `exfat-fuse`, which cannot be told apart;
       - anything not recognised;
       - every folder on Windows.
     - **Finding the type:** one `libc::statfs` call per device, which asks the operating system what kind of file system a folder is on.
       - macOS reads `f_fstypename`.
       - Linux reads `f_type`, checked against `<linux/magic.h>`.
       - The call sits in one small function with its own `#[allow(unsafe_code)]` and a comment, as `service.rs:618` does.
     - **A wrong entry in this table can only make catch-up slower, or make it miss arrivals.** It can never make it move a file that did not arrive, because names decide and times only let a listing be skipped. Say so in the code comment.
   - **`CatchUpRecord`**, using `serde`, in exactly the format of §3b:
     - `load(root) -> Loaded | Missing | Unreadable(reason) | ForAnotherFolder`;
     - `save(&record)`, through e3's write-then-swap helper;
     - `record_path(root)`.
     - An unknown `format_version` counts as `Unreadable`.
     - When loading, drop every entry that lies inside a deeper watched folder, because that folder has its own record.
   - **`Check::new(root, record_snapshot, filters, deeper_roots, recursive, limits)`**, then **`step(&mut reader, budget, clock, stop) -> Working | Finished(CheckResult)`**. It visits a parent before its children. For each folder:
     1. **Read its details first** (rule I2: always before listing).
        - If that fails with "not found", drop the entry and every entry beneath it.
        - For any other error, keep the entry, report it once (`catch_up_folder_unreadable`), and queue nothing.
     2. **List it only if any of these is true:**
        - it has no entry;
        - its times are `null`;
        - `recheck` is set;
        - `owed` is set;
        - either of its times differs from the entry;
        - its drive is `ListEveryFolder`.

        Otherwise, go straight on to its recorded subfolders.
     3. **When it is listed, split the entries into four groups:**
        - **subfolders.** Record all of their names in `subfolders`. Walk only those that pass the skip rules, are not links, and are not deeper watched folders;
        - **clutter**, which is ignored;
        - **names still arriving**, which are neither recorded nor arrivals;
        - **the rest.**

        Then:
        - **Gained names** are the rest, minus the recorded names.
        - Only gained names that pass the `watch.include_extensions` and `exclude_extensions` lists count as arrivals. Those lists are applied here, not when recording, so changing them later does not make old names look new.
        - A folder with gained arrivals becomes a `Folder` item.
        - A folder that only lost names, or gained only clutter or downloads, is **updated at once**: it has been dealt with, and nothing arrived.
     4. **A walkable subfolder whose name is not in the parent's recorded `subfolders` is new.**
        - List it and everything beneath it.
        - If anything in it is an arrival, the **topmost** new folder becomes a `Subtree` item.
        - Either way, every new folder gets an entry at once (rule I1). If nothing in it arrived, the entry holds its real times and names. Otherwise it has times `null` and `owed` set.
     5. **A walkable subfolder named in `subfolders` but with no entry of its own** is recorded as a starting point: it is listed and given entries, and it produces no items (rule I1).
     6. **Dropping entries.** An entry is dropped, with everything beneath it, when its parent was listed and it is not in that listing. **Nothing is dropped for a folder that was not visited.** A stopped check keeps what it did not see.
     7. **The recheck rule.** When an entry's times are written, set `recheck: true` if either time is:
        - less than 5 seconds before the moment the times were read; or
        - later than that moment.

        This covers HFS+'s whole seconds (checked), FAT's 2-second steps and Linux's coarse timestamps. The wall clock only decides *whether to look again*. It never decides whether something arrived.
   - **`CheckResult`** holds:
     - `items`: the folders and subtrees to queue;
     - the updated and dropped entries;
     - `arrived_files` and `arrived_folders`;
     - `recorded_names` and `lost_names`;
     - `folders_checked`, `folders_listed` and `seconds`;
     - `lists_every_folder` and `looked_different`;
     - the folders that could not be read.
   - **`looked_different`** is true when `recorded_names ≥ 200` and `lost_names × 2 > recorded_names`. Both numbers are constants, and are tested at their edges.
   - **Limits:** above 1,000,000 files or 200,000 folders, the check stops and returns `TooBig`. Both are constants that tests can lower.

3. **The "dealt with" rule.** Also in `catch_up.rs`, as pure functions that the organiser calls.
   - **Before a queued folder is organised,** keep its details and its listing.
     - Use the listing e1 took to decide the folder was ready, if it is available; otherwise take a fresh one.
     - Details first (rule I2).
   - **After it is organised,** read its details again, then list it.
     - **The entry's names become:** the names listed now that were either in the "before" listing, or are destinations this organise wrote into this same folder (a rename in place).
     - **`owed` is set if any of these is true:**
       - a name now present is in neither of those groups, and counts as an arrival. It came in while the folder was being organised;
       - the outcome held the folder back (c2);
       - the folder was paused: Test Mode, or the output folder missing (e2);
       - the lock was busy (e3);
       - the organise was stopped part-way (d2).

       Otherwise `owed` is cleared.
     - Files **left alone** ("would be renamed again", a conflict, a failed move) still count as dealt with.
   - **Destinations in other folders.** For each moved file, add its name to the entry of the folder it was moved into, if that folder is inside a watched folder. If that folder has no entry, create one with times `null`.
     - So the organiser's own output never looks like an arrival at the next start.
     - The times stay as they were. So that folder is still listed once next time (cheap), and anything else that arrived there is still seen.

4. **Wiring it into `watch.rs`:**
   1. **Load the records.** First run e2's checks before starting, take e3's `organiser.lock`, and start the watcher. (d1 step 6 already starts the watcher before any walk, so nothing that arrives during the check is missed.) Then load one record per watched folder, by real path, with duplicates removed.
   2. **The start question.** d1's start question gains the sentence in §3f. Whether this is a first run is known before the question, from whether the record file exists.
   3. **Run the check.** The event loop runs `Check::step` with a 250 ms budget, once per pass, until every watched folder has been checked.
      - Events are still received and queued between steps.
      - **Folders under a watched folder that is still being checked are queued, but not organised,** until its check finishes (§3e).
   4. **When a watched folder's check finishes,** act on the result:
      - **`Missing`:** record the starting point (§3c).
      - **`looked_different`:** record a new starting point, queue nothing, and add a `catch_up_skipped` attention item.
      - **`Unreadable`:** rename the file to `<name>.unreadable` (replacing any older one), record a new starting point, and add an attention item.
      - **`TooBig`:** switch catch-up off for that folder, write no record, and add an attention item.
      - **Otherwise:**
        - put every item into e1's queue, with "now" as its last-change time. If e1's `record` cannot take a folder directly, add `enqueue(folder, kind, now)` to e1's queue; do not fake a file event;
        - mark the items `owed` in memory;
        - apply the updated and dropped entries.

      Report per §3f.
   5. **After each organised folder,** apply step 3, whatever put it in the queue: a live event or catch-up.
   6. **`--organize-existing`, when the sweep is accepted and this is not a dry run.**
      - Do not queue catch-up items: the sweep covers every folder.
      - Build the record from the sweep's own listing: one walk, not two.
      - The sweep's folders count as dealt with, through step 3.
      - If the sweep is declined, or run under `--dry-run`, catch-up runs as normal.
      - **One walker.** d1's sweep and this check must use the same skip rules and the same reader. If d1 built its own discovery, switch it to this stage's reader; as a minimum, share the rules. Never two sets of rules.
   7. **Saving. Never under `--dry-run`.**
      - **When to save:**
        - when the record has changed, at most once every 60 seconds;
        - straight after each watched folder's check;
        - after the event loop ends, before "Watcher stopped".
      - **Before each save:**
        - move every event already waiting in the channel into the queue;
        - then mark as `owed` every folder that is queued, held back, waiting on the lock or paused, creating entries where missing (rule I1).
      - **A save that fails** (disk full, permissions) keeps the old file, is reported once (`catch_up_record_not_saved`), and is tried again at the next save.
   8. **Ctrl+C.** The check tests d2's stop flag between folders.
   9. **Reporting.** The human lines, JSON lines and status fields in §3f.

5. **In `service_cmd.rs`:**
   - `meedya service status` shows the catch-up block and the new attention kinds.
   - **A new subcommand, `meedya service reset-catch-up [FOLDERS…]`.** It records what is in the watched folders now as the new starting point, and **moves nothing**. The folders come from settings, or from the command line.
     - **It takes `organiser.lock`.** If an organiser is running, it refuses with exit code 1: *"An organiser is running (process N, started HH:MM), so the catch-up record cannot be reset now. Stop it first (meedya service stop, or Ctrl+C in its window). Nothing was changed."*
     - **Under `--dry-run`** it prints what it would record, and writes nothing.
     - **The switched-off safety catch does not block it,** because it moves nothing.
     - **Why it exists:**
       - A user who has just put files into a watched folder by hand, and does not want them organised, needs a way to say "these are not arrivals".
       - Before stage g no real organiser can run. So this is the only way a hand check can create a record.

6. **Schemas:**
   - **`config/schemas/organiser-catchup.schema.json`:**
     - a `description` on every property;
     - the required fields marked;
     - `names` and `subfolders` constrained to the pattern `^([0-9a-f]{16})*$`;
     - the `owed` values listed;
     - `$comment` notes for maintainers about rules I1 to I6.
   - **A drift test** modelled on `settings_schema_properties_match_appconfig` (`mm-core/src/config/mod.rs:1805`).
   - **e3's `organiser-status.schema.json`:** add the `catch_up` fields and the new attention kinds, and extend its drift test.

7. **(Recommended to move into e1, which is not built yet.) The organiser's own moves are not arrivals while it is running, either.** See §6, e1 addition 1. If e1 does not take it, this stage only makes sure catch-up does not repeat the problem (step 3, destinations).

**Decisions, and what was rejected:**
- **Names decide; times only save work.** The reasons and the evidence are in §3a. Rejected:
  - file dates (copies keep them; checked);
  - the change time (hidden labels bump it, checked; it means nothing on FAT or exFAT, checked);
  - identity numbers (made up on some drives, and reused after a deletion);
  - names plus each file's size and date (one detail-read per file, and what it adds is not an arrival);
  - for now, macOS's change history (§8).
- **List every folder on FAT, exFAT, Windows and unknown file systems.**
  - The Mac's own drivers did update folder times (checked).
  - But FAT and exFAT drives are exactly the ones other devices write to while the Mac is off: cameras, Windows computers. Whether those update folder times was not checked.
  - Rejected: trusting those drives, which would silently miss those arrivals.
  - Rejected: detecting the file system without the `libc` call, for example by reading the output of `mount`. That is fragile.
- **The whole folder is organised, as the running organiser does.** This is owner decision A (§7).
  - Rejected: organising only the new files. One folder would then behave differently depending on whether the organiser happened to be running when the file arrived. It would also need a second path through `scan`.
- **Removals alone do not trigger catch-up.** Nothing arrived.
- **Everything goes through e1's queue; nothing is organised directly.** The queue is the only path with the "still arriving" protections.
- **No second question on a terminal.**
  - d1 asks a numbered question before its sweep, because the sweep can move the whole library.
  - Catch-up only does what the running organiser would have done, and the start question already says so.
  - Nothing moves before the settle wait (about a minute by default), and Ctrl+C during that wait stops everything.
- **"Looks very different" records a new starting point automatically, and says so loudly.**
  - Rejected: refusing at every start until somebody acts. In the background nobody may read the refusal, so catch-up would quietly stay off.
  - The cost: arrivals in that same stretch are not organised. The message says how to organise them by hand.
- **One file per watched folder, in the settings folder, as JSON with a schema.** Rejected:
  - one file for all folders: a terminal run given some other folder would disturb the service's record;
  - a file inside the watched folder: never write into the user's folders;
  - a database: that is #220's job. The record should move into the library database once it exists.
- **No switch to turn catch-up off.** Nobody asked for one, and `reset-catch-up` covers "do not organise what is there now".
- **Previews read the record but never write it.** The reason is the same as in e3: a preview must not change what a real service does.

**Tests to write first.** In brackets: how each fails on the code this stage starts from, which is c1 to e3 landed.

*In `crates/mm-core/src/catch_up.rs`* (all are *compile failures*, because the module is new):
1. `name_fingerprints_never_change`.
   - `""` → `cbf29ce484222325`
   - `"a"` → `af63dc4c8601ec8c`
   - `"foobar"` → `85944171f73967e8`
   - `"Björk.flac"`, in UTF-8 → `0cdd348313fb87ed`

   These were computed with an independent script.
2. `an_unchanged_folder_is_not_listed`: a counting reader shows one detail-read per recorded folder, and **no listing** for folders whose times match.
3. `a_new_file_name_queues_its_folder` and `a_removed_file_alone_queues_nothing_and_updates_the_entry`.
4. New and known folders:
   - `a_new_folder_with_an_arrival_is_queued_as_one_subtree` (the topmost new folder only);
   - `a_new_folder_holding_only_downloads_is_recorded_without_them`;
   - `a_known_folder_without_its_own_entry_is_recorded_not_caught_up`. This covers a nested watched folder removed from settings, and recursion switched on.
5. `an_owed_folder_is_queued_even_when_unchanged`.
6. `clutter_is_never_an_arrival`: `.DS_Store`, `._song.mp3`, `Thumbs.db`, `DESKTOP.INI`, `Icon\r`, and a `_MeedyaManager` copy.
7. `names_still_arriving_are_not_recorded_and_count_once_finished`. Cases:
   - `movie.mkv` beside `movie.mkv.aria2`;
   - `x.flac.part`;
   - an aria2 folder download (`Album/` beside `Album.aria2`);
   - a file inside `x.download/`.
8. Folders that are never walked:
   - `skipped_folders_are_never_walked`: `.Trashes`, `x.download`, `$RECYCLE.BIN`, `System Volume Information`, `lost+found`;
   - `a_linked_folder_is_not_followed` (macOS and Linux only);
   - `a_deeper_watched_folder_keeps_its_own_record`, including dropping the outer record's entries for it when loading.
9. `the_include_list_decides_arrivals_but_not_what_is_recorded`.
   - With `include_extensions: ["flac"]`, a gained `cover.jpg` queues nothing, and a gained `x.flac` does.
   - Then clear the list: `cover.jpg` must **not** now look new.
10. `the_first_check_records_a_starting_point_and_queues_nothing`.
11. `a_folder_that_looks_very_different_is_not_caught_up`. The edges:
    - 200 recorded, 101 lost → skipped;
    - 200 recorded, 100 lost → not skipped;
    - 199 recorded, 199 lost → not skipped.
12. `the_recheck_rule_catches_changes_within_the_same_second` (the HFS+ case).
    - Times less than 5 s old set `recheck`, and so do times in the future.
    - A folder with `recheck` is listed next time, even with identical times.
13. Drives:
    - `drives_that_cannot_be_trusted_have_every_folder_listed`, with the trust answer injected;
    - `file_system_names_are_classified`: a pure table covering `apfs`, `msdos`, `exfat`, the Linux magic numbers and an unknown one.
    - On macOS only, also check that the test's own temporary folder reads as `apfs`. Skip if it does not.
14. Stopping and pacing:
    - `a_stop_request_ends_the_check_between_folders`;
    - `a_stopped_check_keeps_what_it_did_not_see`;
    - `each_step_stays_within_its_time_budget`, with an injected clock.
15. `a_vanished_folder_is_dropped_with_everything_beneath_it` and `a_folder_that_cannot_be_read_is_kept_and_reported`.
16. Loading and saving:
    - `the_record_survives_a_round_trip`, including a folder name that is not valid text (macOS and Linux only);
    - `an_unknown_format_version_is_unreadable`;
    - `a_record_for_another_folder_is_ignored`;
    - `an_unreadable_record_is_set_aside`.
17. `the_record_is_replaced_whole`: no temporary file is left behind, and when the write fails (injected), the old file is intact.
18. The "dealt with" rule:
    - `names_are_what_was_there_before_plus_this_runs_own_renames`;
    - `a_file_that_appeared_while_organising_leaves_the_folder_owed`;
    - `held_paused_lock_busy_and_stopped_folders_stay_owed`;
    - `left_alone_files_count_as_dealt_with`.
19. `a_huge_tree_switches_catch_up_off`, with the limits lowered.
20. `the_record_matches_its_schema`: the drift test.

*In `crates/mm-core/src/watcher/mod.rs`:*

21. `the_name_rules_agree_with_the_file_rules`: for a table of names and siblings, `is_arriving_by_name` must agree with `is_download_in_progress` on real temporary files. *(A compile failure. It guards against the two drifting apart.)*

*In `crates/mm-cli/src/commands/watch.rs`* (all under `ConfigDirGuard`; no test waits for a real file event):

22. **`a_file_that_arrived_while_stopped_is_organised_at_the_next_start`.**
    - Template: `<Extension>/<Filename>`.
    - Write a record for the root with `serde_json::json!` and a few-line local FNV-1a helper. It needs entries `""` and `in`:
      - `""` lists `in` in its `subfolders`;
      - `in` has no names, and times of `0`, so it gets listed.
    - Key the file by the root's real path (`std::fs::canonicalize`), as e2 resolves it.
    - Put `in/song.wav` on disk.
    - Start the organiser, and drive the settle queue past "ready" with e1's injected clock, through the same test entry points the d1 and e1 tests use.
    - Expect `wav/song.wav`.
    - ***Fails by behaviour*** on the starting code: nothing reads the record, so nothing is organised.
23. **`the_first_real_start_records_a_starting_point_and_moves_nothing`.**
    - Files present, no record.
    - After start-up and settling: nothing has moved, **and** `organiser-catchup/<key>.json` exists.
    - ***Fails by behaviour*** on the starting code: no record file is written.
24. `a_preview_never_writes_the_catch_up_record`. *(A guard. It passes on the starting code.)*
25. `a_held_back_rip_is_offered_again_at_every_start_until_complete`: two starts, with a `.cue` whose `.bin` is missing. The folder is still `owed` after both.
26. `test_mode_leaves_caught_up_folders_owed`, with `TestModeEnvGuard`.
27. `left_alone_files_are_not_retried_at_every_start`, with the template `x <Filename>`.
    - The first start reports "would be renamed again".
    - The second queues nothing and reports nothing.
28. `the_organisers_own_moves_are_not_caught_up_next_time`.
    - Move `in/a.wav` to `wav/a.wav`, and stop before `wav` settles.
    - Start again: `wav` is listed but not queued.
29. `a_save_marks_queued_folders_owed_after_taking_waiting_events`.
30. `folders_under_a_folder_still_being_checked_wait_for_the_check`.
31. `organize_existing_replaces_catch_up_and_leaves_a_full_record`.
32. `stopping_during_a_long_check_is_prompt_and_leaves_a_whole_record`.
    - Use `run_until` with a dry-run context, about 2,000 generated folders, and a stop after 200 ms.
    - It must return within 5 seconds, and any record file present must still parse.

*In `crates/mm-cli/src/commands/service_cmd.rs`:*

33. The new subcommand. *(Compile failures: it does not exist yet.)*
    - `reset_catch_up_records_a_starting_point_and_moves_nothing`;
    - `reset_catch_up_refuses_while_an_organiser_is_running`, holding `organiser.lock` in the test;
    - `reset_catch_up_under_dry_run_writes_nothing`.
34. `service_status_shows_catch_up_and_what_needs_attention`, from a hand-written `organiser-status.json` with a `catch_up` block.
    - ***Fails by behaviour*** on the starting code if e3's reader ignores fields it does not know. Otherwise it is a compile failure.
    - Say which, in the test's doc comment.

*In e3's status module:*

35. Its schema drift test, extended. *(It fails until the schema has the new fields.)*

**Hand check** (the §6b harness):
- **Every organiser run uses `--dry-run`.**
- **The only command run without `--dry-run` is `meedya service reset-catch-up`.**
  - It moves no file.
  - It writes only into the throwaway settings folder `$H/config`, just as §6b's harness already writes `settings.json5` there, and e3's hand check writes a status file there.
  - The script proves both, by comparing the scratch tree and the settings folder before and after.
- No service is installed.

```bash
#!/bin/bash
# Hand check for stage e4 (catch-up). Moves nothing: every organiser run is
# --dry-run. `service reset-catch-up` moves no file and writes only into the
# throwaway settings folder $H/config. Everything is inside the scratchpad.
set -u
W="/abs/path/to/.claude/worktrees/org-e4"; B="$W/target/debug/meedya"
S="<session scratchpad>"; H="$S/hand-e4"; rm -rf "$H"; mkdir -p "$H/config" "$H/lib"
export MM_CONFIG_DIR="$H/config"; HR="$(cd "$H" && pwd -P)"
cat > "$H/config/settings.json5" <<EOF
{ rename: { template: "<Extension>/<Filename>" }, watch: { folders: ["$HR/lib"] } }
EOF
tree_state() { find "$H/lib" | sort; }                                           # every path under lib
rec_state()  { find "$H/config" -type f -exec shasum {} + 2>/dev/null | sort; }  # the settings folder
organise() {  # a dry-run organiser: wait for the catch-up line, then for settling
  timeout -s INT 90 "$B" --dry-run watch --organize --settle-secs 1 > "$H/out-$1.txt" 2>&1 & P=$!
  for i in $(seq 1 200); do grep -q "Catch-up" "$H/out-$1.txt" && break; sleep 0.25; done
  sleep 5; kill -INT "$P"; wait "$P"; echo "exit code: $?"; cat "$H/out-$1.txt"; }

mkdir -p "$H/lib/wav" "$H/lib/old"; printf x > "$H/lib/wav/a.wav"; printf x > "$H/lib/old/b.wav"
# (a) First start, no record, preview. Expect "first run … a real run would record … (2 files in
#     3 folders)", no "Would move", and no organiser-catchup folder afterwards.
organise a; ls "$H/config/organiser-catchup" 2>&1
# (b) Reset under --dry-run writes nothing.
R0="$(rec_state)"; "$B" --dry-run service reset-catch-up; [ "$R0" = "$(rec_state)" ] && echo "dry-run wrote nothing"
# (c) Reset for real. Expect "Recorded a starting point … 2 files in 3 folders. Nothing was moved."
T0="$(tree_state)"; "$B" service reset-catch-up; echo "exit code: $?"; [ "$T0" = "$(tree_state)" ] && echo "nothing moved"
# (d) "Arrivals while stopped", plus clutter and a download still in progress.
mkdir -p "$H/lib/new" "$H/lib/dl"; printf x > "$H/lib/new/c.wav"; printf x > "$H/lib/old/d.wav"
printf x > "$H/lib/wav/.DS_Store"; printf x > "$H/lib/wav/Thumbs.db"
printf x > "$H/lib/dl/e.mkv"; printf x > "$H/lib/dl/e.mkv.aria2"
R1="$(rec_state)"; organise d; [ "$R1" = "$(rec_state)" ] && echo "preview left the record alone"
# Expect "Catch-up: 2 files arrived in 2 folders", then "Would move" for new/c.wav, old/d.wav and
# old/b.wav (the whole folder is organised: owner decision A). Nothing for a.wav, .DS_Store,
# Thumbs.db or e.mkv.
# (e) Looks very different: 300 files, reset, then swap in 300 differently named ones.
rm -rf "$H/lib"; mkdir -p "$H/lib/m"; for i in $(seq -w 1 300); do printf x > "$H/lib/m/f$i.wav"; done
"$B" service reset-catch-up
mv "$H/lib" "$H/lib.gone"; mkdir -p "$H/lib/n"; for i in $(seq -w 1 300); do printf x > "$H/lib/n/g$i.wav"; done
organise e   # expect "Catch-up skipped … looks very different … 300 of the 300 files", and no "Would move"
# (f) Budget: 150,000 empty files in 16,251 folders (about 90 s to make), then time the check.
rm -rf "$H/lib" "$H/lib.gone"; mkdir -p "$H/lib"
/usr/bin/python3 - "$H/lib" <<'PY'
import os, sys
r = sys.argv[1]
for a in range(1250):
    for b in range(12):
        d = os.path.join(r, f"Artist {a:04d}", f"Album {b:02d}"); os.makedirs(d)
        for c in range(10): open(os.path.join(d, f"{c+1:02d} - Track.flac"), "w").close()
PY
sleep 6; time "$B" service reset-catch-up   # the first-run cost: expect roughly 10–20 s on this Mac
for a in $(seq 0 9); do printf x > "$H/lib/Artist 000$a/Album 00/99 - New.flac"; done
organise f   # expect "10 files arrived in 10 folders (Checked 16,251 folders in ≤ 2 s; listed about 10)"
# (g) Ctrl+C during a long check. With no record, the preview has to list every folder.
rm -rf "$H/config/organiser-catchup"
timeout -s INT 120 "$B" --dry-run watch --organize > "$H/out-g.txt" 2>&1 & P=$!
for i in $(seq 1 80); do grep -q "Catch-up: checking" "$H/out-g.txt" && break; sleep 0.25; done
sleep 1; kill -INT "$P"; S0=$(date +%s); wait "$P"; echo "exit code: $? after $(( $(date +%s) - S0 )) s"
cat "$H/out-g.txt"   # expect "Stopping…", then "Watcher stopped", exit code 0, within about a second
rm -rf "$H"
```

- **Settling.** After e1, settling takes about two settle periods. So `organise` waits 5 seconds with `--settle-secs 1`.
- **Waiting.** If the orchestrator runs this in the background, it sets a watchdog on the output file, as §6b says. Step (f) takes a few minutes.
- **Optional (h), only if the orchestrator is content to mount a disk image.** I did this safely while planning.
  - Make a 64 MB FAT32 image in the scratchpad: `hdiutil create -size 64m -fs "MS-DOS FAT32"`.
  - Attach it with `-nobrowse -mountpoint "$H/fat"`, point the settings at `$H/fat/lib`, and run (c) and (d).
  - The lines should say every folder is listed "because it is on a FAT or exFAT drive".
  - Detach and delete it afterwards.
- **Cannot be hand-checked before stage g** (say so in the report):
  - a real start saving the record by itself;
  - a real catch-up moving files;
  - saving when stopping.

  Tests 22, 23, 25 to 29 and 31 cover them.

**Done when:**
- the gate (§6a) is clean;
- the tests have been shown failing first, with 22 and 23 (and 34, if it applies) failing by behaviour;
- the hand check's output is saved, and the budget line from (f) is at or under 2 seconds. If it is over, report the measured figure rather than quietly changing the budget;
- a review round finds nothing real;
- the handoff records the number of rounds, and that the reviewer was the same model if Opus built it.

---

## 6. Changes to the rest of the plan

**§1, the plan on one page.** Add a row:

| Stage | What it does, in plain words | Main files | Can run beside |
| --- | --- | --- | --- |
| **e4** | Catches up on files that arrived while the organiser was not running, without sweeping the library. The first run records a starting point | new `mm-core/src/catch_up.rs`, `watcher/mod.rs`, `watch.rs`, `organiser_status.rs`, `service_cmd.rs`, new schema | f1 |

Change the order line to: `c1 → c2 → d1 → d2 → e1 → e2 → e3 → e4 → f2 → Codex review of the whole organiser → g`.

**§3a, the defect table.** Add a row:

| # | Defect, in plain words | Where | Holds? | Stage |
| --- | --- | --- | --- | --- |
| O34 | Files that arrive while the organiser is not running are never organised until something else changes in their folder | Nothing catches up, once d1 makes the sweep opt-in | Yes (owner decision 5, 2026-09-25) | e4 |

**§6c, dependencies.** Replace the diagram with:

```text
review/round2 + gap-fix commit + Codex round-2 fixes land on the working branch
   │
   ├── f1  (help pages + one test)                  ── side by side with everything below
   ├── d3  (service.rs, service_cmd.rs)             ── already on org/line
   │
   └── c1 ─► c2 ─► d1 ─► d2 ─► e1 ─► e2 ─► e3 ─► e4 ─► f2 ─► Codex organiser review ─► g
                                     ▲       ▲      │
                                     │       │      └ needs e1 (the queue), e2 (real paths, Test
                                     │       │        Mode pause), e3 (organiser.lock, status file)
                                     └ d3    └ d3
```

Under it, add: *"e4 changes `watch.rs`, and e3's status file and schema, so it comes strictly after e3. It can run beside f1 only."*

**§7, owner decisions.** Decision 3 is **answered**: the owner chose to build catch-up (decision 5 of 2026-09-25).
- Replace its text with a pointer to e4.
- Add the two new decisions from §7 below.

**Stage e1 (not built yet): three recommended additions.** Each is small, and each closes a gap the existing plan leaves open.
1. **The organiser's own moves should not count as arrivals.**
   - Today, and as e1 is written, moving `in/a.wav` to `Artist/Album/a.wav` raises an event in `Artist/Album`.
   - That folder then settles and is organised **in full**, so any older, never-organised files sitting there are organised too.
   - This happens at c1 today: `organise_settled` groups every settled path by its folder.
   - It quietly breaks d1's promise that existing files are left alone.
   - **The fix:** an event whose path is exactly a destination this run has just written (per `MoveHistory`) does not queue its folder. Other events in that folder still do.
2. **The Windows and Linux twins of O19.** Skip `$RECYCLE.BIN`, `System Volume Information` and `lost+found`, as well as hidden folders, through the shared `is_skipped_folder_name` (e4 step 1).
3. **The `--organize-existing` sweep should wait for settling too.**
   - d1's sweep organises each folder straight away. So a file being copied in at start-up can be organised half-written.
   - Putting the sweep's folders through e1's queue, as e4 does for catch-up, closes this.

If e1 does not take them:
- e4 does (1) for catch-up only (step 3);
- e4 does (2) in full (step 1);
- (3) stays open.

Say so in the handoff.

**Stage e3.** No change to what it builds, but e4 adds fields to its status file and schema.
- e3's reader must not reject fields it does not know: no `deny_unknown_fields`.
- e4's new fields get `#[serde(default)]`.
- Then both old and new status files parse.

**Stage f2 ("Must cover").** Replace *"files that arrived while it was stopped are not organised until something in their folder changes (owner decision 3)"* with the following:
- catch-up: what it does, what counts as "arrived", and the first run;
- **what it cannot catch:**
  - a file replaced under the same name, or edited in place;
  - arrivals in a watched folder that looks very different;
- that the **whole folder** is organised (owner decision A);
- FAT and exFAT drives, Windows and unrecognised file systems list every folder at each start, which is slower;
- network drives are best effort;
- `meedya service reset-catch-up`, and when to use it;
- where the record lives, and that deleting it simply makes the next start a first run;
- the catch-up block in `service status`, and its attention items;
- the `catch_up` JSON line, in `docs/api/cli.md`.

f2's check for phrases that must not appear already includes `organised once now` (the old sweep's wording). Keep it.

**Stage g.**
- **Condition 1** gains e4.
- **The Codex scope** gains:
  - `catch_up.rs`;
  - the watcher's name rules;
  - the `reset-catch-up` subcommand;
  - `organiser-catchup.schema.json`;
  - rules I1 to I6, as things to try to break.
- **Hand check (1)**, the end-to-end dry-run script, includes `hand-e4.sh`.
- **Hand check (2)**, the real run confined to the scratchpad (only with the owner's OK), gains a catch-up round:
  - start, then stop with Ctrl+C;
  - drop a tagged file into the scratch folder;
  - start again: expect *"1 file arrived"*, and the file organised;
  - start once more: expect *"nothing arrived"*.
- **The `run_until` switch-on test** also checks that a real start writes the catch-up record.

**Issues to file:**
- the catch-up issue itself (§5, "Issue");
- e1 addition 1, if e1 does not take it, as an issue of its own: *"Organiser: files already in a destination folder are organised when the organiser moves something into it"*.

---

## 7. Decisions only the owner can take

Nothing in e4 waits for these. The recommended answer is what gets built meanwhile.

**A. When something arrives in a folder, should the whole folder be organised?**
- The running organiser organises a whole folder when a file in it settles. So older, never-organised files in the same folder are organised too.
- Catch-up does the same, so the two behave alike.
- Example: a new track dropped into `Old Album/` also moves the old tracks in `Old Album/` into place.
- *Recommended: yes. Keep the two alike, and say so plainly in the help pages.*
  - Organising only the new files would be a bigger change, to the running organiser as well. It belongs with #220's record of each file's history.
  - Separately, the organiser's own moves into a destination folder should **not** count as an arrival there (§6, e1 addition 1). That is a gap, not a choice.

**B. What counts as "arrived".**
- Catch-up judges arrivals by **new file names**.
- A file replaced by another of the same name, or edited in place while the organiser was stopped, is not caught up.
- Such a file is organised the next time something else in its folder changes, or with `--organize-existing`.
- *Recommended: accept.* Catching those too would mean reading every file's details at every start. On this Mac that was about 40 times slower than reading only the folders' details, and the changes it would catch are not arrivals.

**Decisions this stage has taken that you might want to reverse** (built as described):
- **When a watched folder looks very different** (at least 200 files recorded, more than half gone), catch-up organises nothing, records a new starting point and tells you. Arrivals in that same stretch are not organised automatically.
- **FAT and exFAT drives** (and Windows, and unrecognised drives) have every folder listed at each start. This is slower, but it does not trust folder times that other devices may not keep.
- **Network drives are trusted** to keep folder times, as best effort.
- **A new command:** `meedya service reset-catch-up`.
- **Catch-up is switched off** for a watched folder holding more than 1,000,000 files or 200,000 folders.
- **No second question on a terminal** before catching up. The start question covers it.

---

## 8. Not now: suggestions

- **Use macOS's own change history (FSEvents) as a speed-up.**
  - Keep the record as the truth. But on macOS, ask the operating system which folders changed since the last save, and skip reading the rest.
  - That would turn "one read per folder" into "one read per change".
  - It needs low-level calls (`fsevent-sys` is already in the lock file, through `notify`), and a fallback whenever the history is incomplete.
  - It is not needed to meet the budget on the internal disk.
- **Recognise NTFS on Windows** through `GetVolumeInformationW`, so Windows stops listing every folder. This needs the `fileapi` feature of `winapi`, and testing on Windows.
- **A final save on the service's stop signal.**
  - d2 decided against a graceful stop, and catch-up is safe without one.
  - Adding it would mean fewer folders re-examined after each logout.
- **Move the record into the library database** once #220 exists.
- **A compressed record** (gzip, about a third of the size), if records ever grow large. `format_version` allows the change.

---

## 9. What I could not verify

- **Speeds straight after a restart.** Only warm timings were measured, taken straight after the tree was made. A first start after a reboot may be slower. The hand check measures the real program, but also warm.
- **Why listing a folder costs about 1 ms on this Mac.** Most likely the endpoint-security extension (NordVPN Shield) inspecting every folder opened, but that is not proven. On a Mac without such a tool, listing is probably much cheaper, which only helps.
- **Whether Windows, cameras and other devices update folder times** when they write to a FAT or exFAT drive. Only the Mac's own drivers were checked, and they do. This is why such drives are always listed in full.
- **Linux:**
  - that ext4 and the others update a moved file's change time, and a folder's times, as expected;
  - that the `statfs` magic numbers are right (check them against `<linux/magic.h>`);
  - that Linux's FAT driver hands out fresh identity numbers.

  All of this comes from documentation and kernel source; none of it was run. A wrong entry in the file-system table can only make catch-up slower, or make it miss arrivals (§5 step 2).
- **Network drives.** How SMB and NFS keep and cache folder times was not tested. Catch-up there is best effort.
- **Whether `std::fs::canonicalize` on macOS returns the on-disk letter case** for a folder typed in a different case.
  - If it does not, `~/music` and `~/Music` would get two records, and the second would start with a first run.
  - That is safe: nothing is organised, and one catch-up is lost.
- **The names of e1's, e2's and e3's functions.** None of those stages is built yet. This plan uses the names their plans give; the builder follows what lands.
- **That Rust's `DirEntry::file_type()` avoids reading each entry's details on APFS.** It should, because the file system reports the type in the listing. It was not measured from Rust.
- **Windows as a whole:**
  - there is no change time;
  - how Windows names are encoded in the record is untested.

  Nothing Windows-specific can be built or run on this Mac. CI covers it at stage g.

