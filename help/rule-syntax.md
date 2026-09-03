# 📐 Rule & Template Syntax Guide — MeedyaManager

> **(C) 2025–2026 MWBM Partners Ltd**

MeedyaManager's rule engine uses a template syntax inspired by [MusicBee's template system](https://musicbee.fandom.com/wiki/Templates), extended with 16 numbered custom tags, video support, and audio characteristic detection.

> **Implemented in Milestone 3 (v1.2-M3).** The template syntax described below — tag references and the 24 functions listed here — is fully functional and matches `crates/mm-core/src/rule_engine/functions.rs`. Legacy `{placeholder}` syntax from M1/M2 is still supported but deprecated — templates are auto-detected.

---

## 📋 Table of Contents

1. [Basic Syntax](#basic-syntax)
2. [Tag References](#tag-references)
3. [Functions](#functions)
4. [Logical Functions](#logical-functions-6)
5. [String Functions](#string-functions-8)
6. [Numeric / Formatting Functions](#numeric--formatting-functions-4)
7. [Lookup Functions](#lookup-functions-3)
8. [MeedyaManager Extension Functions](#meedyamanager-extension-functions-3)
9. [Path Construction](#path-construction)
10. [Examples](#examples)

---

## Basic Syntax

A template combines three elements:

- **Tag references** — `<TagName>` — replaced with the metadata value
- **Functions** — `$FunctionName(args)` — process and transform values
- **Literal text** — plain characters (folders separators, dashes, spaces, etc.)

### Simple Example

```text
<Album Artist>/<Album>/<Track #> - <Title>.<Extension>
```

Produces: `Pink Floyd/The Wall/01 - In The Flesh.mp3`

---

## Tag References

Tags are enclosed in angle brackets and replaced with the corresponding metadata value.

> The tables below are the **exact** tag set registered in
> `crates/mm-core/src/rule_engine/tag_registry.rs`. There is no separate set of
> video-specific tags (`<Show>`, `<Season>`, `<Episode>`, `<Director>`, `<Resolution>`,
> etc. do not exist) — video files are described using the same standard tags where the
> underlying container has them, plus the classification tags below.

### Standard Metadata Tags

| Tag | Description | Example |
| --- | ----------- | ------- |
| `<Title>` | Track title | "Bohemian Rhapsody" |
| `<Artist>` | Track artist(s) | "Queen" |
| `<Album>` | Album name | "A Night at the Opera" |
| `<Album Artist>` (or `<AlbumArtist>`) | Album-level artist | "Queen" |
| `<Year>` (or `<Date>`) | Release year | "1975" |
| `<Genre>` | Genre(s) | "Rock" |
| `<Track #>` (or `<Track Number>`, `<TrackNumber>`) | Track number | "11" |
| `<Disc #>` (or `<Disc Number>`, `<DiscNumber>`) | Disc number | "1" |
| `<Track Count>` (or `<TrackTotal>`) | Total tracks on disc | "12" |
| `<Disc Count>` (or `<DiscTotal>`) | Total discs in set | "1" |
| `<Composer>` | Composer | "Freddie Mercury" |
| `<Label>` | Record label | "EMI" |
| `<Comment>` | Comment field | "Remastered 2011" |
| `<Lyrics>` | Lyrics | — |
| `<ISRC>` | International Standard Recording Code | "GBUM71029601" |
| `<Barcode>` | Release barcode | "5099902987524" |
| `<Catalog#>` (or `<Catalog Number>`) | Catalogue number | "CDP 7 46001 2" |
| `<Compilation>` | Compilation flag | "1" |
| `<BPM>` | Beats per minute | "72" |

There is no `<Publisher>` alias for the record label — use `<Label>` (the extended
`<Publisher>` tag below is a separate, independent metadata field).

### Extended Metadata Tags

| Tag | Description |
| --- | ----------- |
| `<Sort Title>`, `<Sort Artist>`, `<Sort Album>`, `<Sort Album Artist>`, `<Sort Composer>` | Sort-order variants of the corresponding tag |
| `<Grouping>` | Grouping field |
| `<Conductor>` | Conductor |
| `<Remixer>` | Remixer |
| `<Producer>` | Producer |
| `<Lyricist>` | Lyricist |
| `<Mood>` | Mood |
| `<Initial Key>` (or `<Key>`) | Musical key |
| `<Encoder>` | Encoder tool |
| `<Copyright>` | Copyright string |
| `<Publisher>` | Publisher (distinct from `<Label>`) |
| `<Language>` | Language |
| `<Rating>` | Rating |
| `<Subtitle>` | Subtitle |

### Classification Tags

Computed at evaluation time from the media classifier (`crates/mm-core/src/classify`), not
read from file metadata:

| Tag | Description | Example Values |
| --- | ----------- | -------------- |
| `<Media Group>` | Top-level classification | Audio, Video, Image, Document, Archive |
| `<Media Class>` | What the file actually is | Music, Podcast, Audiobook, Movie, TVShow, MusicVideo, Concert |
| `<Media Format>` | Container/codec classification | MP3, FLAC, AAC, WAV, AIFF, ALAC, OGG |
| `<Media Quality>` | Quality tier | Lossless, HiRes, Lossy320, Lossy256, Lossy192, Lossy128, LossyLow |

### Audio Property Tags

Read from the decoded audio stream, not from tags — `""` if the property is unavailable:

| Tag | Description | Example Values |
| --- | ----------- | -------------- |
| `<Bitrate>` | Bitrate in kbps | "320", "1411" |
| `<Sample Rate>` | Sample rate in Hz | "44100", "96000" |
| `<Channels>` | Channel count | "2", "6", "8" |
| `<Bit Depth>` | Bit depth | "16", "24", "32" |
| `<Duration>` | Human-readable duration | "3:42" |
| `<Duration Secs>` (or `<DurationSecs>`) | Duration in whole seconds | "222" |

There are no `<Codec>`, `<Channel Layout>`, `<Spatial Format>` or `<Multichannel>` tags —
codec/channel-layout naming and spatial-audio detection (Dolby Atmos, Sony 360 Reality
Audio, Apple Spatial Audio) are not implemented (see [Supported Formats](supported-formats.md)
and issue [#131](https://github.com/MWBMPartners/MeedyaManager/issues/131)).

### File Tags

| Tag | Description | Example |
| --- | ----------- | ------- |
| `<Filename>` | Original filename (no extension) | "01 - Song" |
| `<Extension>` | File extension (no dot) | "mp3" |
| `<Folder>` | Immediate parent directory name | "A Night at the Opera" |
| `<Full Path>` (or `<Fullpath>`, `<File Path>`, `<Filepath>`) | Full absolute file path | "/Downloads/song.mp3" |

There are no `<File Size>` or `<Date Added>` tags.

### Custom Tags

MeedyaManager provides exactly **16 numbered custom tag slots** — `<Custom1>` through
`<Custom16>` — for your own values (SpotifyURL, MusicBrainzID, a personal rating, etc.):

```text
<Custom1>
<Custom7>
<Custom16>
```

There is no free-form `<Custom:Name>` syntax and no way to add a 17th slot — see
[Custom Tags](custom-tags.md) for the full picture.

---

## Functions

All functions are prefixed with `$` and use parentheses for arguments; function names are
case-insensitive (`$If`, `$if` and `$IF` are equivalent). This is the **complete, exhaustive
set of 24 functions** implemented in `crates/mm-core/src/rule_engine/functions.rs:120-153` —
calling anything else fails with `unknown template function`.

A value is **truthy** for the logical functions below unless it is empty, `"0"`, or `"false"`
(case-insensitive); everything else, including a tag that resolved to a non-empty string, is
truthy. There is no built-in `=`/`>`/`<` comparison operator inside `$If` — build the
condition first with `$Contains`, `$IsMatch` or `$IsNull`, as shown in the examples below.

### Logical Functions (6)

#### `$If` — Conditional Evaluation

```text
$If(condition, trueResult, falseResult?)
```

Returns `trueResult` if `condition` is truthy, otherwise `falseResult` (defaults to `""` if
omitted).

```text
$If($Contains(<Genre>, Rock), Rock/<Artist>, Other/<Artist>)
```

#### `$And` — All Values Truthy

```text
$And(value1, value2, ...)
```

Returns the **last** value if every argument is truthy; otherwise returns `""`.

```text
$If($And($Contains(<Genre>, Rock), <Year>), Modern Rock, Other)
```

#### `$Or` — First Truthy Value

```text
$Or(value1, value2, ...)
```

Returns the first truthy argument; otherwise returns `""`.

```text
$If($Or($Contains(<Genre>, Rock), $Contains(<Genre>, Metal)), Rock & Metal/<Artist>, Other/<Artist>)
```

#### `$Not` — Negate

```text
$Not(value)
```

Returns `"1"` if `value` is falsy; `""` if it is truthy.

```text
$If($Not($IsNull(<Comment>)), Has Comment, No Comment)
```

#### `$IsNull` — Test for Empty Tag

```text
$IsNull(value)
```

Returns `"1"` if `value` is an empty string; `""` otherwise.

```text
$If($IsNull(<Album Artist>), <Artist>, <Album Artist>)
```

Falls back to `<Artist>` when `<Album Artist>` is empty.

#### `$Contains` — Substring Check

```text
$Contains(haystack, needle)
```

Case-insensitive. Returns `"1"` if `haystack` contains `needle`; `""` otherwise.

```text
$If($Contains(<Genre>, Rock), It's Rock, Not Rock)
```

---

### String Functions (8)

#### `$Replace` — Find and Replace

```text
$Replace(string, search, replacement)
```

Replaces every occurrence of `search` in `string` with `replacement`.

```text
$Replace(<Artist>, &, and)
```

#### `$Upper` — Uppercase

```text
$Upper(string)
```

```text
$Upper(<Genre>)
```

Returns `"ROCK"` for "Rock".

#### `$Lower` — Lowercase

```text
$Lower(string)
```

```text
$Lower(<Extension>)
```

Returns `"mp3"` for "MP3".

#### `$Left` — First N Characters

```text
$Left(string, n)
```

Clamps to the string's length.

```text
$Left(<Artist>, 1)
```

Returns `"Q"` for "Queen".

#### `$Right` — Last N Characters

```text
$Right(string, n)
```

Clamps to the string's length.

```text
$Right(<Year>, 2)
```

Returns `"75"` for "1975".

#### `$Mid` — Substring

```text
$Mid(string, start, length?)
```

Returns the substring starting at the 0-indexed `start` position, for `length` characters
(or to the end of the string if `length` is omitted).

```text
$Mid(<Title>, 6, 5)
```

Returns `"World"` for "Hello World".

#### `$Trim` — Remove Whitespace

```text
$Trim(string)
```

Strips leading and trailing whitespace.

```text
$Trim(<Title>)
```

#### `$Split` — Split and Pick

```text
$Split(string, separator, index)
```

Splits `string` on `separator` and returns the 0-indexed element at `index`; returns `""` if
`index` is out of range.

```text
$Split(<Artist>, "; ", 0)
```

For "Artist A; Artist B" returns "Artist A".

---

### Numeric / Formatting Functions (4)

#### `$Pad` — Pad to a Minimum Width

```text
$Pad(string, width, fill_char?)
```

Left-pads `string` to at least `width` characters using `fill_char` (default `"0"`).

```text
$Pad(<Track #>, 2)
```

Returns `"01"` for track 1, `"12"` for track 12.

#### `$Date` — Current Date/Time

```text
$Date(format?)
```

Returns the **current local date/time** (not a tag value) formatted with a
[chrono strftime pattern](https://docs.rs/chrono/latest/chrono/format/strftime/index.html);
default format is `%Y-%m-%d`.

```text
$Date(%Y)
```

Returns the current 4-digit year, e.g. `"2026"`.

#### `$Format` — Format a Number

```text
$Format(number, decimals?)
```

Parses `number` as a floating-point value and formats it to `decimals` places (default `0`).
Fails if the input is not numeric.

```text
$Format(<BPM>, 1)
```

#### `$Count` — Count Multi-Value Items

```text
$Count(string, separator?)
```

Counts the items when `string` is split on `separator` (default `"; "`); returns `"0"` for an
empty string.

```text
$Count(<Genre>)
```

For "Rock; Progressive Rock" returns `"2"`.

---

### Lookup Functions (3)

#### `$Sort` — Sort Multi-Value Items

```text
$Sort(string, separator?)
```

Alphabetically sorts the items in a `separator`-delimited string (default `"; "`). This does
**not** strip leading articles ("The", "A", "An") from artist names — it is a plain
alphabetical sort of the delimited items.

```text
$Sort(<Genre>)
```

For "Rock; Blues" returns "Blues; Rock".

#### `$IsMatch` — Regex Pattern Check

```text
$IsMatch(string, regex_pattern)
```

Returns `"1"` if `string` matches `regex_pattern` (compiled patterns are cached); `""`
otherwise.

```text
$If($IsMatch(<Title>, "^[A-Z]"), Starts with letter, Other)
```

#### `$Lookup` — Built-In Table Lookup

```text
$Lookup(key, table_name)
```

Looks `key` up in one of two built-in tables and returns "" on a miss — there is no way to
add custom tables. `table_name` must be one of:

- `genre_folder` — maps genre names (e.g. `rock`, `hip hop`, `jazz`) to a folder label
  (`Rock`, `Hip-Hop`, `Jazz`, …)
- `quality_folder` — maps quality labels (e.g. `lossless`, `320 kbps`) to a folder label
  (`Lossless`, `High Quality`, …)

```text
$Lookup(<Genre>, genre_folder)
```

For "Rock" returns "Rock"; for an unmapped genre like "Polka" returns `""`.

---

### MeedyaManager Extension Functions (3)

These three take their value from the rule engine's evaluation context rather than tag
arguments.

#### `$MediaClass` — Classification Class

```text
$MediaClass()
```

Returns the current file's `MediaClass` (e.g. "Music", "Movie", "TV Show") from the
evaluation context; returns `""` if no classification is available.

#### `$MediaGroup` — Classification Group

```text
$MediaGroup()
```

Returns the current file's `MediaGroup` (e.g. "Audio", "Video") from the evaluation context;
returns `""` if no classification is available.

#### `$FirstValue` — First Multi-Value Item

```text
$FirstValue(string, separator?)
```

Returns the first item when `string` is split on `separator` (default `"; "`).

```text
$FirstValue(<Genre>)
```

For "Rock; Progressive Rock" returns "Rock".

---

## Path Construction

Folder separators in templates create directory structure:

```text
<Media Class>/<Album Artist>/<Album>/<Title>.<Extension>
```

Use `/` as the separator — MeedyaManager automatically converts to the correct OS path separator.

### Nested Folders

You can nest as deeply as needed:

```text
Library/<Media Group>/<Media Quality>/<Genre>/<Album Artist>/<Album> (<Year>)/<$Pad(<Track #>,2)> - <Title>.<Extension>
```

Produces something like: `Library/Audio/Lossless/Rock/Queen/A Night at the Opera (1975)/11 - Bohemian Rhapsody.flac`

---

## Examples

### Basic Music Organisation

```text
Music/<Album Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Extension>
```

### Lossless vs Lossy Separation

`$If` has no built-in `=` comparison — build the condition with `$Contains` first:

```text
$If($Contains(<Media Quality>, Lossless),
    Music/Lossless/<Album Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Extension>,
    Music/Lossy/<Album Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Extension>
)
```

`<Media Quality>` is `Lossless` or `HiRes` for lossless files, so this also matches
high-resolution audio into the "Lossless" branch — use `$Or` to add more matches if needed.

### Movies

```text
Movies/<Title> (<Year>)/<Title>.<Extension>
```

### A-Z Folder Grouping

```text
Music/$Upper($Left(<Album Artist>, 1))/<Album Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Extension>
```

Produces: `Music/Q/Queen/A Night at the Opera/11 - Bohemian Rhapsody.flac`

This groups strictly by the first character of `<Album Artist>` as stored in the tag — there
is no built-in stripping of leading articles ("The", "A", "An"), so "The Beatles" would file
under `T`, not `B`.

### Handle Missing Album Artist

```text
$If($IsNull(<Album Artist>),
    Music/Unknown Artist/<Album>/<Title>.<Extension>,
    Music/<Album Artist>/<Album>/<Title>.<Extension>
)
```

### Media-Type Router

Only `<Media Group>` and `<Media Class>` are available to route on — there are no
show/season/episode tags, so a video branch can only use the tags that exist for it (Title,
Year, and the classification tags):

```text
$If($Contains(<Media Group>, Audio),
    Music/<Album Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Extension>,
    $If($Contains(<Media Class>, Movie),
        Movies/<Title> (<Year>)/<Title>.<Extension>,
        Video/<Media Class>/<Title>.<Extension>
    )
)
```

---

> 📝 *This syntax is fully implemented as of M3 (v1.2-M3). Use
> `meedya rule validate "<template>"` to check a template's syntax without evaluating it,
> `meedya rule test "<template>" <file>` to evaluate it against a real file, and
> `meedya rule tags` to list every known tag name and its type.*
