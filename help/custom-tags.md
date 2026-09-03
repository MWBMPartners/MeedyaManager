# Custom Tags — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

MeedyaManager's rename-template engine recognises **16 numbered custom tag slots** —
`Custom1` through `Custom16` — for your own values. There is no free-form named custom tag
syntax, and no limit-raising option: the slot count is fixed at 16.

> ⚠️ **Status: not yet functional.** `Custom1`–`Custom16` are recognised as valid tag *names*
> by the rule engine's template parser (`crates/mm-core/src/rule_engine/tag_registry.rs:241-256`),
> but nothing in MeedyaManager currently reads or writes their values. `meedya edit`'s
> `--set`/`--remove` flags and the automatic metadata reader both work only against the fixed
> set of ~44 keys in `crates/mm-core/src/metadata/mod.rs`'s tag mapping table — `Custom1`
> through `Custom16` are not among them. In practice, `<Custom1>` (etc.) in a rename template
> always evaluates to an empty string today, and there is no CLI or GUI action that stores a
> value into one of these slots. This page describes what the template syntax recognises, not
> a working feature — treat it as a preview of where custom tags are headed, not something you
> can rely on yet.

---

## Table of Contents

1. [What Are Custom Tags?](#what-are-custom-tags)
2. [Referencing Custom Tags in Templates](#referencing-custom-tags-in-templates)
3. [Why You Can't Set a Value Yet](#why-you-cant-set-a-value-yet)
4. [What Does Work Today](#what-does-work-today)

---

## What Are Custom Tags?

Custom tags are meant to be user-defined metadata slots that go beyond the standard set
(Artist, Album, Title, etc.) — for example, tracking who ripped a file, where the source
media is stored, or a personal rating. MeedyaManager reserves exactly 16 such slots,
`Custom1` through `Custom16`, rather than supporting arbitrary named tags. There is also an
undocumented `meedyameta.*` prefix recognised by the same lookup (e.g. `meedyameta.rating`),
but it has the identical limitation described below — no write or read path exists for it
either.

---

## Referencing Custom Tags in Templates

The template parser accepts `<Custom1>` through `<Custom16>` as tag references:

```text
<Artist>/<Album>/<Title> [<Custom1>].<Extension>
```

You can validate that a template parses correctly with:

```bash
meedya rule validate "<Artist>/<Album>/<Title> [<Custom1>]"
```

But because no value is ever stored against `Custom1`, this template will always resolve
with an empty bracket — `Artist/Album/Title [].mp3` — never a real value. `meedya rule test`
against a real file will show the same result:

```bash
meedya rule test "<Custom1>" song.mp3
```

---

## Why You Can't Set a Value Yet

`meedya edit` writes metadata through `mm_core::metadata::write_tags`, which only writes keys
present in its internal `tag_key_mappings()` table (`crates/mm-core/src/metadata/mod.rs:389-446`)
— roughly 44 standard fields (Title, Artist, Album, Composer, ReplayGain fields, podcast
fields, and so on). Any other key, including `custom_1`, is **silently ignored**:

```bash
# This command exits successfully but writes nothing — "custom_1" is not
# a recognised key, so write_tags drops it without an error.
meedya edit song.mp3 --set custom_1=2024
```

The same is true in reverse: `extract_tags` only reads back keys from that same table, so
even a custom TXXX/Vorbis-comment/atom field written by another application (e.g. MusicBee)
will not be picked up by MeedyaManager's tag reader or shown by `meedya debug`.

---

## What Does Work Today

If you need to store extra information on a file right now, use one of the standard
extended-metadata fields that MeedyaManager *does* read and write — `<Comment>`, `<Grouping>`,
and `<Mood>` are all wired through `tag_key_mappings()` and behave normally with
`meedya edit --set` / `--remove` and in rename templates. See
[Rule & Template Syntax](rule-syntax.md#extended-metadata-tags) for the full list — not every
tag the template parser recognises is backed by a working read/write path yet, so when in
doubt, test with `meedya edit <file> --set <key>=<value>` followed by `meedya debug <file>`
to confirm the value round-trips.
