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
