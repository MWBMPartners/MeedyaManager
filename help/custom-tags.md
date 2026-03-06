# Custom Tags — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

MeedyaManager supports unlimited custom metadata tags. Unlike MusicBee's limit of 16–20 custom tags, you can define as many as you need. Custom tags are written to standard tag containers (ID3v2, Vorbis Comments, MP4 atoms) and are readable by most media players.

---

## Table of Contents

1. [What Are Custom Tags?](#what-are-custom-tags)
2. [Using Custom Tags in Templates](#using-custom-tags-in-templates)
3. [Reading Custom Tags](#reading-custom-tags)
4. [Writing Custom Tags](#writing-custom-tags)
5. [Custom Tag Format by Container](#custom-tag-format-by-container)

---

## What Are Custom Tags?

Custom tags are user-defined metadata fields that go beyond the standard set (Artist, Album, Title, etc.). Use them to store any information you want to track — for example:

- `Ripped By` — who digitized the file
- `Storage Location` — physical location of the original media
- `Purchase Date` — when you bought it
- `Personal Rating` — your own rating separate from the standard rating field
- `Collection` — which collection this belongs to
- `Remaster Year` — year of the remaster edition

---

## Using Custom Tags in Templates

Reference custom tags in rename templates using the `<Custom:Name>` syntax:

```json5
rename: {
  template: "<Artist>/<Album>/<Title> [<Custom:Remaster Year>].<Ext>"
}
```

Use `$If` to handle cases where the custom tag might not be present:

```json5
rename: {
  template: "<Artist>/<Album>/$If(<Custom:Remaster Year>, <Title> [<Custom:Remaster Year>], <Title>).<Ext>"
}
```

Test your template:

```bash
meedya rule test --template "<Artist>/<Custom:Remaster Year>/<Title>" song.mp3
```

---

## Reading Custom Tags

Custom tags are read automatically during file inspection. To see all tags including custom ones:

```bash
meedya debug path/to/file.mp3
```

In JSON format (useful for scripting):

```bash
meedya debug path/to/file.mp3 --json
```

---

## Writing Custom Tags

Use `meedya edit` to write a custom tag:

```bash
# Write a custom tag
meedya edit song.mp3 --tag "Custom:Remaster Year=2024"
meedya edit song.mp3 --tag "Custom:Ripped By=Alice"

# Write multiple custom tags at once
meedya edit song.mp3 \
  --tag "Custom:Remaster Year=2024" \
  --tag "Custom:Storage Location=Shelf A"

# Remove a custom tag
meedya edit song.mp3 --remove-tag "Custom:Remaster Year"

# Preview without writing
meedya edit song.mp3 --tag "Custom:Remaster Year=2024" --dry-run
```

---

## Custom Tag Format by Container

Custom tags are stored differently depending on the file format's tag container:

| Format | Container | Custom Tag Storage |
| ------ | --------- | ------------------ |
| MP3 | ID3v2 | `TXXX` frame with description = tag name |
| FLAC, OGG, Opus | Vorbis Comments | Uppercase field name, e.g. `CUSTOM:REMASTER YEAR` |
| M4A, MP4, M4V | iTunes atoms | `----:com.mwbm:TagName` custom atom |
| WMA | ASF | Extended Content Description object |
| APE, MPC | APEv2 | Plain key-value pair |

These are standard mechanisms — files with custom tags will remain playable and the tags will be visible in most music players (though some may show the raw field name rather than a friendly label).
