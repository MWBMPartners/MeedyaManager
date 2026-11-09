# Background Service — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

MeedyaManager can run as a persistent background service that starts automatically and monitors your media folders continuously, even when you're not actively using the application.

> ## ⚠️ Switched off in this build (2026-09-23)
>
> The developers found two ways the automatic organiser could damage files — see
> [why "skip" is forced](#why-skip-is-forced) for one of them, and the short version is: it
> could separate a disc image from the cue sheet that belongs with it, or rename the same file
> over and over. Until that is fixed and checked, real organising is switched off, and with it
> the background service:
>
> - **`meedya service install` now refuses on every platform, including Linux and macOS**,
>   and installs nothing. (It already refused on Windows for a different, permanent reason —
>   see [Windows](#windows-not-available-yet).)
> - **`meedya watch --organize` without the global `--dry-run` flag refuses too**, and moves
>   nothing — even if you pass `--yes`.
> - **Previews still work.** `meedya --dry-run service install` still prints what it would
>   register, and `meedya --dry-run watch --organize` still previews what it would move,
>   because a preview does not touch anything.
> - **`service uninstall`, `service start`, `service stop` and `service status` are
>   unaffected.** If you installed the service from an earlier build, run
>   `meedya service uninstall` to remove it.
>
> Everything else on this page describes how the feature works and will work again once it is
> switched back on.

---

## Table of Contents

1. [Overview](#overview)
2. [What actually happens](#what-actually-happens)
3. [The settle window: why it waits](#the-settle-window-why-it-waits)
4. [Why "skip" is forced](#why-skip-is-forced)
5. [Only one copy moves files at a time](#only-one-copy-moves-files-at-a-time)
6. [Installing the Service](#installing-the-service)
7. [Starting and Stopping](#starting-and-stopping)
8. [Checking Service Status](#checking-service-status)
9. [Uninstalling the Service](#uninstalling-the-service)
10. [Platform Details](#platform-details)
    - [Linux (systemd)](#linux-systemd)
    - [macOS (launchd)](#macos-launchd)
    - [Windows (not available yet)](#windows-not-available-yet)
11. [Troubleshooting](#troubleshooting)

---

## Overview

The background service runs `meedya watch --organize --yes` continuously
(`crates/mm-core/src/service.rs`). Once installed, it:

- Starts automatically when you log in (Linux/macOS — the only two platforms this can be
  installed on today; see [Windows](#windows-not-available-yet) below)
- Monitors all folders configured in `settings.json5`
- Renames and moves new media files into place as they settle — see
  [What actually happens](#what-actually-happens) below for the step-by-step version

**What it will not do.** It will not touch a file that is still being copied in — it waits for
the file to go quiet first (see [the settle window](#the-settle-window-why-it-waits)). It will
not use your configured "rename" conflict handling — it always falls back to the safer "skip"
behaviour instead, and says why below. And if another copy of MeedyaManager (a manual
`meedya scan --execute`, or the desktop app's Execute button) is moving files at the same
moment, the service waits its turn rather than racing it — see
[Only one copy moves files at a time](#only-one-copy-moves-files-at-a-time).

The service is managed via the `meedya service` subcommand — no manual editing of systemd unit files, plist files, or Windows registry entries is required.

---

## What actually happens

Step by step, once the service (or `meedya watch --organize`) is running:

1. **A sweep of every watched folder runs once, straight away.** A watcher only hears about
   changes that happen while it is running, so anything that arrived overnight, or while the
   service was stopped, would otherwise sit there untouched forever. The start-up sweep is what
   makes "install it and forget about it" actually true (`crates/mm-cli/src/commands/watch.rs`,
   `Organiser::sweep_roots`).
2. **New files are watched for, but not acted on straight away.** A file being copied in fires a
   stream of change notifications while it is still half-written. Organising it there and then
   would mean reading tags out of an incomplete file, so each file has to sit untouched for a
   short "settle window" first — see below.
3. **Once a file has settled, its whole folder is organised, not just that one file** — ten
   tracks dropped into one album folder become one piece of work, not ten
   (`crates/mm-cli/src/commands/watch.rs`, `group_by_parent`). Only that folder is looked at, not
   its subfolders, so one arriving file never triggers a rescan of your whole library.
4. **The result is always laid out under the watched folder itself**, never under whatever
   subfolder the file happened to land in. A file dropped into
   `<watched folder>/incoming/song.mp3` ends up at
   `<watched folder>/Artist/Album/Title.mp3`, not buried under `incoming/`. This is fixed
   behaviour for the watcher — the `--output-dir` flag and the `rename.output_dir` setting in
   `settings.json5` are not consulted here, because there would be no single sensible directory
   to send them to if they were (`crates/mm-cli/src/commands/watch.rs`, `organise_directory`).
5. **Templates and rename rules come from your settings file**, exactly as they do for
   `meedya scan --execute` — there is no separate configuration for the watcher.

---

## The settle window: why it waits

By default, a file has to go two seconds without any further change before the watcher will
touch it (`--settle-secs`, default `2` —
`crates/mm-cli/src/commands/watch.rs::WatchArgs::settle_secs`). Change it with, for example:

```bash
meedya watch --organize --settle-secs 5
```

The reason for waiting at all: while an application is still copying a file in, the operating
system reports a stream of "this file changed" notifications. If MeedyaManager organised the
file on the first one, it would be reading tags — and moving — a file that is not finished
being written yet. Waiting until nothing has happened to the file for a few seconds is a cheap,
reliable way to tell "finished arriving" from "still being written", without needing the
operating system's own file-locking APIs.

---

## Why "skip" is forced

Whatever your `settings.json5` says under `conflict_strategy`, the watcher and the service
always behave as if it were set to `"skip"` while organising — and if your setting really was
something else, it says so once, the first time it matters:

```text
conflict_strategy "rename" is not used while watching — a file that is already at its
destination would be renamed again on every pass, so the watcher always skips conflicts
instead.
```

Here is why. Picture two files that are genuinely different but happen to carry identical tags
— two different recordings both tagged "Title: Intro", say. With `conflict_strategy = "rename"`,
the second one is renamed to `Intro (1).mp3` the first time it is organised. But the watcher
re-checks that same folder every time anything in it settles — and when it recomputes where
`Intro (1).mp3` belongs, from its tags alone, it gets `Intro.mp3` again, which is now taken by
the other file. So it gets renamed again, to `Intro (2).mp3`, and the cycle repeats on every
future pass. Run by hand, somebody notices after the second file appears. Left to a service that
runs unattended from login until shutdown, it does not stop on its own. (This is a real,
separately tracked bug in the counter-renaming logic itself — issue #224 — and forcing "skip"
here is a deliberate way of containing it, not a claim that it is fixed.)

With "skip", the file that is already correctly named is left alone, and a genuine second file
that collides with it is left where it is rather than renamed — nothing is lost, and nothing
loops.

---

## Only one copy moves files at a time

MeedyaManager will not let two things move the same files at once — the background service, a
`meedya scan --execute` you run by hand, and the desktop app's Execute button all share the same
lock. If the service tries to organise a folder while something else is already mid-move, it
does not fight over the files: it leaves them queued and tries again after another settle
window has passed. This never blocks the watcher itself — it only ever affects organising, and
only for as long as the other copy is actually busy. See the "Only one copy may move files at a
time" section of [cli-reference.md](cli-reference.md#meedya-scan) for the full detail on the
lock itself.

---

## Installing the Service

> **Currently refused everywhere.** As explained at the top of this page, `meedya service
> install` is switched off while the organiser's known problems are fixed — see
> [Switched off in this build](#-switched-off-in-this-build-2026-09-23). It will print an
> error and exit with code `3`, and nothing will be installed. The description below is how it
> works once it is switched back on.

```bash
meedya service install
```

This registers MeedyaManager with your operating system's service manager, using the currently
running `meedya` binary — that path is worked out and written into the service definition at
install time, so if you move or reinstall `meedya` afterwards, run `service install` again.
**Built for Linux and macOS only** — see [Windows](#windows-not-available-yet) below for why
Windows is different, and what to use instead.

To try it first without registering anything for real:

```bash
meedya --dry-run service install
```

This prints what would be registered and exits without writing anything.

To use a specific binary path (e.g. if you have multiple installations):

```bash
meedya service install --bin-path /opt/meedya/bin/meedya
```

> On **Linux**, this creates a systemd **user** unit file and enables it — the service runs as
> you, not as root.
> On **macOS**, this creates a launchd **LaunchAgent** plist and loads it — the service runs as
> you, not as an administrator.
> On **Windows**, `meedya service install` refuses and explains why — see
> [Windows](#windows-not-available-yet) below.
>
> Both installed services run `meedya watch --organize --yes`. The `--yes` matters: `--organize`
> normally asks once, on an attended terminal, before it starts moving files — a background
> service has no terminal and nobody to answer that question, so it is told in advance to skip
> asking.

**Try it by hand first.** Run this in a terminal against one of your watched folders and watch
what it says it would do, with nothing actually moved:

```bash
meedya watch ~/Music --organize --dry-run
```

Normally you would then drop `--dry-run` to let it move files for real, and install it as a
service once you are happy. **Right now that next step is switched off** — dropping
`--dry-run` will refuse and move nothing, and installing the service will refuse too — until
the organiser's known problems are fixed (see the notice at the top of this page).

---

## Starting and Stopping

```bash
meedya service start    # start the service immediately
meedya service stop     # stop the running service
```

After installing, the service will start automatically the next time you log in. Use `start` to
begin immediately without waiting.

---

## Checking Service Status

```bash
meedya service status
```

Output:

```text
MeedyaManager background service: RUNNING
```

For machine-readable output:

```bash
meedya service status --json
```

```json
{
  "service": "meedyamanager",
  "status": "running",
  "running": true
}
```

Exit code: `0` if running, `1` if stopped, not installed, or status unknown.

---

## Uninstalling the Service

```bash
meedya service stop
meedya service uninstall
```

This removes the service registration from the OS service manager. Your `settings.json5` and media files are not affected.

---

## Platform Details

### Linux (systemd)

The service is installed as a **systemd user unit** — it runs under your user account, not as root.

**Unit file location:**

```text
~/.config/systemd/user/meedyamanager.service
```

**Direct systemd commands (if needed):**

```bash
# View service status
systemctl --user status meedyamanager

# View recent logs
journalctl --user -u meedyamanager -n 50 --no-pager

# Follow logs live
journalctl --user -u meedyamanager -f

# Restart after config change
systemctl --user restart meedyamanager

# Enable auto-start (done automatically by meedya service install)
systemctl --user enable meedyamanager

# Allow the service to run without being logged in (requires lingering)
loginctl enable-linger $USER
```

**Generated unit file (example):**

```ini
[Unit]
Description=MeedyaManager — Media File Auto-Organiser
After=network.target

[Service]
Type=simple
ExecStart=/home/you/.cargo/bin/meedya watch --organize --yes
Restart=on-failure
RestartSec=5s
CPUSchedulingPolicy=idle
IOSchedulingClass=idle

[Install]
WantedBy=default.target
```

The `idle` scheduling lines are deliberate — they tell Linux to only give the service CPU time
when nothing else wants it, so it does not compete with whatever you are actually doing.

---

### macOS (launchd)

The service is installed as a **LaunchAgent** — it runs when you log in, under your user account.

**Plist file location:**

```text
~/Library/LaunchAgents/com.mwbm.meedyamanager.plist
```

**Direct launchctl commands (if needed):**

```bash
# View service status
launchctl list | grep meedyamanager

# Load the service manually
launchctl load ~/Library/LaunchAgents/com.mwbm.meedyamanager.plist

# Unload the service
launchctl unload ~/Library/LaunchAgents/com.mwbm.meedyamanager.plist

# View recent logs (macOS 12+)
log show --predicate 'subsystem == "com.mwbm.meedyamanager"' --last 1h
```

**Generated plist (example):**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.mwbm.meedyamanager</string>
  <key>ProgramArguments</key>
  <array>
    <string>/usr/local/bin/meedya</string>
    <string>watch</string>
    <string>--organize</string>
    <string>--yes</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <dict>
    <key>Crashed</key>
    <true/>
  </dict>
  <key>ProcessType</key>
  <string>Background</string>
</dict>
</plist>
```

`KeepAlive` only restarts the service if it crashed — stopping it yourself with
`meedya service stop` does not trigger a restart. `ProcessType` of `Background` asks macOS to
schedule it at low priority, so it does not compete with whatever you are actively doing.
Standard output and error are logged to `/tmp/meedyamanager.stdout.log` and
`/tmp/meedyamanager.stderr.log`.

**macOS privacy permissions:**

If the service monitors folders in protected locations (Desktop, Downloads, Documents), macOS may prompt for permission. If the prompt doesn't appear automatically:

1. Open **System Settings > Privacy & Security > Files and Folders**
2. Grant MeedyaManager access to the required directories
3. Restart the service: `meedya service stop && meedya service start`

---

### Windows (not available yet)

`meedya service install` refuses on Windows and explains why, rather than registering something
that would not actually work
(`crates/mm-cli/src/commands/service_cmd.rs`):

```text
Installing a background service is not available on Windows yet.

A program registered with `sc create` has to report back to the Windows Service Control
Manager within about thirty seconds of starting. MeedyaManager does not speak that protocol,
so Windows would stop it again almost immediately. It would also run as the LocalSystem
account, which reads a different settings file from yours and may not be able to reach your
media folders at all.

Automatic organising itself is also switched off in this build while known problems are
fixed, so there is nothing to run in the background yet. Once it is switched back on, the way
to do this will be Task Scheduler, running
    meedya watch --organize --yes
at logon. That runs as you and reads your settings.
```

There are two separate, real problems, not one:

1. **The Service Control Manager protocol.** A genuine Windows Service has to check in with
   Windows within about thirty seconds of starting, and keep responding to it afterwards.
   `meedya` is an ordinary console program — it has no idea that protocol exists — so Windows
   would decide it had hung and kill it almost as soon as it started.
2. **The account it would run as.** A service registered the usual way runs as the
   **LocalSystem** account, which is a different account from yours, with its own home
   directory. It would load a different `settings.json5` from the one you can see and edit, and
   it may not even be able to reach your media folders.

**What to use instead: Task Scheduler.** Create a task that runs at logon, running as yourself,
executing:

```text
meedya watch --organize --yes
```

That runs under your own account — your `settings.json5`, your file permissions — and does not
need to speak to the Service Control Manager at all, because Task Scheduler is not that.

> **This command is currently switched off too.** As explained at the top of this page, real
> organising is switched off everywhere until known problems are fixed, so this Task Scheduler
> task would run and immediately refuse (exit code `3`), moving nothing, until that changes.

Because `service install` refuses outright, `service start`, `service stop`, `service uninstall`
and `service status` have nothing to act on: there is no MeedyaManager Windows Service to find,
on this release, on Windows.

---

## Troubleshooting

### Service installed but not starting

1. Check the service status for an error message:

   ```bash
   meedya service status
   meedya -vv watch --dry-run   # run interactively to see startup errors
   ```

2. Inspect the config MeedyaManager will actually load — there is no `meedya config validate`
   command; use `show` instead:

   ```bash
   meedya config show
   ```

3. Ensure the `meedya` binary path hasn't changed since installation. If you've reinstalled or updated MeedyaManager, reinstall the service:

   ```bash
   meedya service uninstall
   meedya service install
   ```

### Service stops unexpectedly

Check platform-specific logs:

```bash
# Linux
journalctl --user -u meedyamanager -n 100

# macOS
log show --predicate 'subsystem == "com.mwbm.meedyamanager"' --last 2h
# or the plain log files it writes directly:
cat /tmp/meedyamanager.stdout.log /tmp/meedyamanager.stderr.log
```

(There is nothing to check here on Windows — see
[Windows (not available yet)](#windows-not-available-yet).)

### Config changes not taking effect

The service loads `settings.json5` at startup. After editing the config, restart the service:

```bash
meedya service stop
meedya service start
```
