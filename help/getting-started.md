# Getting Started with MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

Welcome to MeedyaManager — a cross-platform media file manager and auto-organizer. This guide
walks you through building it, running your first scan, and basic configuration.

> **No installers exist yet.** MeedyaManager has not had a public release — the only GitHub
> release is the pre-rename "MetaMancer v1.0-M1" pre-release from 2025-06-16, and there is no
> `.msix`, `.dmg`, `.deb`, `.rpm`, Flatpak, Snap, or AppImage artefact you can download today.
> Everything below is build-from-source instructions.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Building From Source](#building-from-source)
3. [First Run](#first-run)
4. [Basic Configuration](#basic-configuration)
5. [Metadata Lookup](#metadata-lookup)
6. [Next Steps](#next-steps)

---

## Prerequisites

| Component | Requirement |
| --------- | ----------- |
| **Rust** | 1.85+ — [rustup.rs](https://rustup.rs) |
| **Platform tools** | Linux: `libgtk-4-dev`, `libadwaita-1-dev` (only needed for the `mm-gtk` UI, which is not a workspace member by default — see below); macOS/Windows: none extra for the CLI |
| **OS** | Windows (x64/ARM64), macOS (Apple Silicon only), or Linux (x64/ARM64) |

---

## Building From Source

```bash
# Clone the repository
git clone https://github.com/MWBMPartners/MeedyaManager.git
cd MeedyaManager

# Build the 8-crate Rust workspace (mm-core, mm-cli, mm-providers, mm-cloud,
# mm-export, mm-server, mm-ffi, mm-update)
cargo build --release
```

> **`cargo build --workspace` does NOT build the Linux GTK4 UI.** `mm-gtk` is deliberately
> excluded from the workspace `members` list (it needs the Linux-only `gettextrs` crate), so a
> plain workspace build produces the CLI and libraries only. To build the GTK4 UI on Linux:
>
> ```bash
> cargo build --release -p mm-gtk
> ```
>
> Standalone building of `mm-gtk` outside a full workspace checkout is currently broken — see
> issue [#199](https://github.com/MWBMPartners/MeedyaManager/issues/199).

The CLI binary is produced at:

```text
target/release/meedya           (macOS / Linux)
target\release\meedya.exe       (Windows)
```

To install the CLI binary on your `PATH`:

```bash
cargo install --path crates/mm-cli
```

### Building the platform GUIs

- **macOS:** `macos/Package.swift` is a Swift Package (there is no `.xcodeproj`). It targets
  macOS 15 and needs Xcode 26.3+ / Swift 6.3 to build. Open the folder in Xcode, or run
  `swift build -c release` from `macos/`.
- **Windows:** `windows/MeedyaManager` is a WinUI 3 / .NET project — open the solution in Visual
  Studio 2022+ with the Windows App SDK workload, or build with `dotnet build`.
- **Linux:** see the `mm-gtk` note above.

---

## First Run

### Inspect a Single File

The quickest way to test MeedyaManager is to inspect a media file:

```bash
meedya debug path/to/your/song.mp3
```

This displays all detected metadata, including:

- Media group (Audio / Video / Image / Document)
- Format class (MP3, FLAC, MP4, MKV, etc.)
- Media class (Music, Movie, TV Show, Podcast, etc.)
- Quality type (Lossy / Lossless / Uncompressed)
- All embedded tags (artist, album, title, track number, etc.)

For JSON output (useful for scripting):

```bash
meedya --json debug path/to/song.mp3
```

### Preview Renames for a Directory

Scan a directory and preview what MeedyaManager would rename each file to, without touching
anything:

```bash
meedya scan ~/Music --dry-run
```

> ### ⚠️ Before you ever pass `--execute`
>
> `meedya scan --execute` performs the renames for real, and it has a known data-loss bug
> (issue [#201](https://github.com/MWBMPartners/MeedyaManager/issues/201)): if your rename
> template causes two different source files to resolve to the same destination path, the
> second rename silently overwrites the first — there is no prompt and no backup. This can also
> happen when a folder-shaped template gets flattened. **Always run `meedya scan` in preview
> mode first (the default — no `--execute`, or explicit `--dry-run`) and check the whole preview
> list for duplicate destination paths before you add `--execute` on anything you cannot afford
> to lose.** See [cli-reference.md](cli-reference.md#meedya-scan) for the full warning.

### Start the Folder Watcher

Watch directories for new media files and log what happens. `watch` on its own only **logs**
file-system events — it does not rename or move anything unless you add `--organize`:

```bash
# Log-only — nothing is moved, regardless of --dry-run
meedya watch

# Actually organise files as they arrive (this is the one that renames/moves)
meedya watch --organize

# Preview what --organize would do, without moving files
meedya watch --organize --dry-run
```

> **Tip:** Always run with `--organize --dry-run` first to verify your rules produce the
> expected results before enabling live file operations.

### Launch the GUI

**Linux (GTK4):** build and run `mm-gtk` as shown above (`cargo run --release -p mm-gtk`).

**macOS / Windows:** build the platform app as shown above and launch it from Xcode / Visual
Studio, or the built app bundle, until a packaged release exists.

---

## Basic Configuration

MeedyaManager stores its configuration in a JSON5 file. The location is:

| Platform | Path |
| -------- | ---- |
| **macOS** | `~/Library/Application Support/MeedyaManager/settings.json5` |
| **Linux** | `~/.config/MeedyaManager/settings.json5` |
| **Windows** | `%APPDATA%\MeedyaManager\settings.json5` |

A default configuration is created automatically on first run (or via `meedya config init`). To
view it:

```bash
meedya config show
```

### Minimal Configuration Example

```json5
{
  watch: {
    // Folders to monitor for new media files
    folders: [
      "~/Downloads/Media",
      "~/Desktop/NewMedia"
    ],
    recursive: true
  },

  rename: {
    // Output directory for organised files
    output_dir: "~/Media",

    // Rename template using MusicBee-style <Tag> syntax
    template: "<Media Class>/<Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>",

    // What to do when a destination file already exists
    conflict_strategy: "rename"   // "rename", "skip", or "overwrite"
  }
}
```

See [configuration.md](configuration.md) for the full settings reference — and note that the
example config file shipped in this repo's `config/settings.json5` uses a **different, stale
schema** and does not match this reference; do not copy it.

---

## Metadata Lookup

> **⚠️ Status: not yet implemented.** `meedya lookup` is still a stub (see
> [cli-reference.md](cli-reference.md#meedya-lookup)). It parses its arguments and prints a
> "coming in M5" message; it does not query any provider or write any tag yet, even though the
> M5 milestone issues are closed on GitHub. The examples below show the intended future syntax,
> not working commands.

```bash
# Not yet functional — prints a stub message today
meedya lookup "Never Gonna Give You Up"
meedya lookup "Never Gonna Give You Up" --provider musicbrainz
meedya lookup "Never Gonna Give You Up" --auto
```

The `mm-providers` library underneath already implements 13 working metadata providers (plus 6
disabled stub providers) across music, video, and podcasts — see [providers/](providers/) for
what each one actually does — but nothing in the CLI or GUI wires them up yet.

---

## Next Steps

- **Configure rules:** [rule-syntax.md](rule-syntax.md) — full template syntax reference
- **CLI reference:** [cli-reference.md](cli-reference.md) — every command and option
- **Configuration reference:** [configuration.md](configuration.md) — all settings explained
- **Supported formats:** [supported-formats.md](supported-formats.md)
- **Background service:** [background-service.md](background-service.md) — run MeedyaManager at startup
- **Metadata providers:** [providers/](providers/) — what's real and what's a stub, provider by provider
- **Troubleshooting:** [troubleshooting.md](troubleshooting.md)
- **FAQ:** [faq.md](faq.md)
