# 📐 Rule & Template Syntax Guide — MeedyaManager

> **(C) 2025–2026 MWBM Partners Ltd**

MeedyaManager's rule engine uses a template syntax inspired by [MusicBee's template system](https://musicbee.fandom.com/wiki/Templates), extended with unlimited custom tags, video support, and audio characteristic detection.

> **Implemented in Milestone 3 (v1.2-M3).** The full template syntax described below is fully functional. Legacy `{placeholder}` syntax from M1/M2 is still supported but deprecated — templates are auto-detected.

---

## 📋 Table of Contents

1. [Basic Syntax](#basic-syntax)
2. [Tag References](#tag-references)
3. [Functions](#functions)
4. [Conditional Logic](#conditional-logic)
5. [String Functions](#string-functions)
6. [Splitting Functions](#splitting-functions)
7. [Formatting Functions](#formatting-functions)
8. [Path Construction](#path-construction)
9. [Examples](#examples)

---

## Basic Syntax

A template combines three elements:

- **Tag references** — `<TagName>` — replaced with the metadata value
- **Functions** — `$FunctionName(args)` — process and transform values
- **Literal text** — plain characters (folders separators, dashes, spaces, etc.)

### Simple Example

```
<Album Artist>/<Album>/<Track #> - <Title>.<Ext>
```

Produces: `Pink Floyd/The Wall/01 - In The Flesh.mp3`

---

## Tag References

Tags are enclosed in angle brackets and replaced with the corresponding metadata value.

### Standard Audio Tags

| Tag | Description | Example |
| --- | ----------- | ------- |
| `<Title>` | Track title | "Bohemian Rhapsody" |
| `<Artist>` | Track artist(s) | "Queen" |
| `<Album>` | Album name | "A Night at the Opera" |
| `<Album Artist>` | Album-level artist | "Queen" |
| `<Year>` | Release year | "1975" |
| `<Genre>` | Genre(s) | "Rock" |
| `<Track #>` | Track number | "11" |
| `<Disc #>` | Disc number | "1" |
| `<Total Tracks>` | Total tracks on disc | "12" |
| `<Total Discs>` | Total discs in set | "1" |
| `<Composer>` | Composer | "Freddie Mercury" |
| `<Publisher>` | Record label | "EMI" |
| `<Comment>` | Comment field | "Remastered 2011" |
| `<BPM>` | Beats per minute | "72" |

### Standard Video Tags

| Tag | Description | Example |
| --- | ----------- | ------- |
| `<Show>` | TV show name | "Breaking Bad" |
| `<Season>` | Season number | "5" |
| `<Episode>` | Episode number | "16" |
| `<Episode Title>` | Episode title | "Felina" |
| `<Director>` | Director name | "Vince Gilligan" |
| `<Resolution>` | Video resolution | "1080p" |

### Classification Tags

| Tag | Description | Example Values |
| --- | ----------- | -------------- |
| `<Media Group>` | Level 1 classification | Audio, Video, Image, Book |
| `<Format Class>` | Level 2 classification | MP3, FLAC, MP4, MKV |
| `<Media Class>` | Level 3 classification | Music, Movie, TV Show, Podcast |
| `<Quality Type>` | Level 4 classification | Lossy, Lossless |

### Audio Property Tags

| Tag | Description | Example Values |
| --- | ----------- | -------------- |
| `<Codec>` | Audio codec | AAC, FLAC, ALAC, Vorbis, Opus |
| `<Bitrate>` | Bitrate | "320", "1411" |
| `<Sample Rate>` | Sample rate | "44100", "96000" |
| `<Channels>` | Channel count | "2", "6", "8" |
| `<Channel Layout>` | Channel layout | "Stereo", "5.1", "7.1" |
| `<Spatial Format>` | Spatial audio type | "Dolby Atmos", "360 Reality Audio" |
| `<Multichannel>` | Multichannel format | "Dolby Digital", "Dolby Digital Plus", "DTS" |
| `<Bit Depth>` | Bit depth | "16", "24", "32" |

### File Tags

| Tag | Description | Example |
| --- | ----------- | ------- |
| `<Filename>` | Original filename (no extension) | "01 - Song" |
| `<Ext>` | File extension (no dot) | "mp3" |
| `<Path>` | Original file path | "/Downloads/song.mp3" |
| `<File Size>` | File size in bytes | "8234567" |
| `<Date Added>` | Date file was detected | "2025-06-15" |

### Custom Tags

Custom tags use the `Custom:` prefix and support unlimited user-defined names:

```
<Custom:SpotifyURL>
<Custom:MusicBrainzID>
<Custom:MyRating>
```

No limit on the number of custom tags (unlike MusicBee's 16-20 limit).

---

## Functions

All functions are prefixed with `$` and use parentheses for arguments.

### Conditional Logic

#### `$If` — Conditional Evaluation

```
$If(criteria, trueResult, falseResult)
```

**Criteria operators:** `=`, `>`, `<`

```
$If(<Genre>=Rock, Rock/<Artist>, Other/<Artist>)
$If(<Year>>2000, Modern/<Album>, Classic/<Album>)
```

#### `$And` — Both Conditions True

```
$If($And(<Genre>=Rock, <Year>>2000), Modern Rock, Other)
```

#### `$Or` — Either Condition True

```
$If($Or(<Genre>=Rock, <Genre>=Metal), Rock & Metal/<Artist>, Other/<Artist>)
```

#### `$IsNull` — Handle Missing Tags

```
$IsNull(<Album Artist>, <Artist>, <Album Artist>)
```

Returns `<Artist>` if `<Album Artist>` is empty; otherwise returns `<Album Artist>`.

#### `$Contains` — Substring Check

```
$If($Contains(<Genre>, Rock)=T, It's Rock, Not Rock)
```

Returns `"T"` if the tag contains the search text, `"F"` otherwise.

#### `$IsMatch` — Regex Pattern Check

```
$If($IsMatch(<Title>, "^[A-Z]")=T, Starts with letter, Other)
```

Returns `"T"` if the tag matches the regex pattern, `"F"` otherwise.

---

## String Functions

#### `$Replace` — Find and Replace

```
$Replace(<Artist>, &, and)
```

#### `$RxReplace` — Regex Replace

```
$RxReplace(<Title>, "\s*\(feat\..*?\)", "")
```

Removes "(feat. ...)" from titles.

#### `$Left` — First N Characters

```
$Left(<Artist>, 1)
```

Returns `"Q"` for "Queen".

#### `$Right` — Last N Characters

```
$Right(<Year>, 2)
```

Returns `"75"` for "1975".

#### `$Upper` — Uppercase

```
$Upper(<Genre>)
```

Returns `"ROCK"` for "Rock".

#### `$Lower` — Lowercase

```
$Lower(<Ext>)
```

Returns `"mp3"` for "MP3".

#### `$Trim` — Remove Whitespace

```
$Trim(<Title>)
```

Removes leading and trailing spaces.

---

## Splitting Functions

#### `$Split` — Split Left-to-Right

```
$Split(<Artist>, ;, 1)
```

For "Artist A; Artist B", returns "Artist A".

#### `$RSplit` — Split Right-to-Left

```
$RSplit(<Artist>, " ", 1)
```

For "John Smith", returns "Smith".

#### `$First` — First Multi-Value

```
$First(<Genre>)
```

For "Rock; Progressive Rock", returns "Rock".

---

## Formatting Functions

#### `$Pad` — Zero-Pad Numbers

```
$Pad(<Track #>, 2)
```

Returns `"01"` for track 1, `"12"` for track 12.

#### `$Date` — Format Dates

```
$Date(<Date Added>, yyyy-MM-dd)
```

Format tokens: `yyyy` (year), `MM` (month), `dd` (day), `hh` (hours), `mm` (minutes), `ss` (seconds).

#### `$Sort` — Strip Sort Words

```
$Sort(<Artist>)
```

Returns `"Beatles"` for "The Beatles" (strips "The", "A", "An").

#### `$Group` — Group by Characters

```
$Group(<Artist>, 1)
```

Returns `"Q"` for "Queen" — useful for A-Z folder grouping.

---

## Path Construction

Folder separators in templates create directory structure:

```
<Media Class>/<Album Artist>/<Album>/<Title>.<Ext>
```

Use `/` as the separator — MeedyaManager automatically converts to the correct OS path separator.

### Nested Folders

You can nest as deeply as needed:

```
Library/<Media Group>/<Quality Type>/<Genre>/<Album Artist>/<Album> (<Year>)/<$Pad(<Track #>,2)> - <Title>.<Ext>
```

Produces: `Library/Audio/Lossless/Rock/Queen/A Night at the Opera (1975)/11 - Bohemian Rhapsody.flac`

---

## Examples

### Basic Music Organisation

```
Music/<Album Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>
```

### Lossless vs Lossy Separation

```
$If(<Quality Type>=Lossless,
    Music/Lossless/<Album Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>,
    Music/Lossy/<Album Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>
)
```

### TV Shows

```
TV Shows/<Show>/Season <$Pad(<Season>,2)>/<Show> - S<$Pad(<Season>,2)>E<$Pad(<Episode>,2)> - <Episode Title>.<Ext>
```

Produces: `TV Shows/Breaking Bad/Season 05/Breaking Bad - S05E16 - Felina.mkv`

### Movies

```
Movies/<Title> (<Year>)/<Title>.<Ext>
```

### Spatial Audio Detection

```
$If($Or($Contains(<Spatial Format>, Atmos), $Contains(<Spatial Format>, 360 Reality)),
    Music/Spatial/<Album Artist>/<Album>/<Title>.<Ext>,
    Music/Standard/<Album Artist>/<Album>/<Title>.<Ext>
)
```

### A-Z Folder Grouping

```
Music/$Group($Sort(<Album Artist>),1)/<Album Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>
```

Produces: `Music/Q/Queen/A Night at the Opera/11 - Bohemian Rhapsody.flac`

### Handle Missing Album Artist

```
$IsNull(<Album Artist>,
    Music/Unknown Artist/<Album>/<Title>.<Ext>,
    Music/<Album Artist>/<Album>/<Title>.<Ext>
)
```

### Multi-Type Router

```
$If(<Media Class>=Music,
    Music/<Album Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>,
    $If(<Media Class>=TV Show,
        TV/<Show>/Season <$Pad(<Season>,2)>/<Show> S<$Pad(<Season>,2)>E<$Pad(<Episode>,2)>.<Ext>,
        $If(<Media Class>=Movie,
            Movies/<Title> (<Year>)/<Title>.<Ext>,
            $If(<Media Class>=Podcast,
                Podcasts/<Show>/<Date Added> - <Title>.<Ext>,
                Unsorted/<Filename>.<Ext>
            )
        )
    )
)
```

---

> 📝 *This syntax is fully implemented as of M3 (v1.2-M3). Use `meedyamanager rule --validate --template "..."` to check template syntax from the command line.*
