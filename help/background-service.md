# Background Service — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

MeedyaManager can run as a persistent background service that starts automatically and monitors your media folders continuously, even when you're not actively using the application.

---

## Table of Contents

1. [Overview](#overview)
2. [Installing the Service](#installing-the-service)
3. [Starting and Stopping](#starting-and-stopping)
4. [Checking Service Status](#checking-service-status)
5. [Uninstalling the Service](#uninstalling-the-service)
6. [Platform Details](#platform-details)
   - [Linux (systemd)](#linux-systemd)
   - [macOS (launchd)](#macos-launchd)
   - [Windows (Windows Service)](#windows-windows-service)
7. [Troubleshooting](#troubleshooting)

---

## Overview

The background service runs `meedya watch` continuously. Once installed, it:

- Starts automatically when you log in (user service) or at boot (system service)
- Monitors all folders configured in `settings.json5`
- Renames and organises new media files as they arrive
- Retries files that are locked or in use

The service is managed via the `meedya service` subcommand — no manual editing of systemd unit files, plist files, or Windows registry entries is required.

---

## Installing the Service

```bash
meedya service install
```

This registers MeedyaManager with your operating system's service manager using the currently installed `meedya` binary. The service is set to start automatically on login.

To use a specific binary path (e.g. if you have multiple installations):

```bash
meedya service install --bin-path /opt/meedya/bin/meedya
```

> On **Linux**, this creates a systemd user unit file and enables it.
> On **macOS**, this creates a launchd LaunchAgent plist and loads it.
> On **Windows**, this registers a Windows Service via `sc.exe`.

---

## Starting and Stopping

```bash
meedya service start    # start the service immediately
meedya service stop     # stop the running service
```

After installing, the service will start automatically at your next login. Use `start` to begin immediately without waiting.

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
Description=MeedyaManager — Background Media Organiser
After=network.target

[Service]
Type=simple
ExecStart=/home/you/.cargo/bin/meedya watch
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

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
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
</dict>
</plist>
```

**macOS privacy permissions:**

If the service monitors folders in protected locations (Desktop, Downloads, Documents), macOS may prompt for permission. If the prompt doesn't appear automatically:

1. Open **System Settings > Privacy & Security > Files and Folders**
2. Grant MeedyaManager access to the required directories
3. Restart the service: `meedya service stop && meedya service start`

---

### Windows (Windows Service)

The service is registered as a **Windows Service** using `sc.exe`. It runs under your user account and starts at login.

**Direct sc.exe commands (if needed):**

```cmd
:: Check service status
sc query MeedyaManager

:: Start manually
sc start MeedyaManager

:: Stop
sc stop MeedyaManager

:: View in Services console
services.msc
```

**Windows Defender / Antivirus:**

If Windows Defender flags the service on first run, add an exclusion for the MeedyaManager install directory in **Windows Security > Virus & threat protection > Exclusions**.

---

## Troubleshooting

### Service installed but not starting

1. Check the service status for an error message:

   ```bash
   meedya service status
   meedya -vv watch --dry-run   # run interactively to see startup errors
   ```

2. Verify your config is valid:

   ```bash
   meedya config validate
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

# Windows (Event Viewer)
eventvwr.msc   # Application > Source: MeedyaManager
```

### Config changes not taking effect

The service loads `settings.json5` at startup. After editing the config, restart the service:

```bash
meedya service stop
meedya service start
```
