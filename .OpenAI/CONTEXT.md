# MeedyaManager — context for Codex

> **(C) 2025-2026 MWBM Partners Ltd**
>
> Codex's own short orientation note, kept alongside Claude's `.claude/` folder at the owner's
> request (2026-09-23). It points to the full documents rather than copying them, so there is
> only one place that can go out of date.

## Read these, in this order

1. **`.claude/HANDOFF.md` §0** — where the work stands right now: the branch, what is committed,
   what is broken, what is owed, and what to do next. It records only checked facts.
2. **`AGENTS.md`** (repository root) — the rules for AI coding tools, including the owner's
   standing rules of 2026-09-23.
3. **`.OpenAI/MEMORY.md`** — the lessons and facts most likely to trip up a new session.
4. **`.claude/CLAUDE.md`** — the full project rules. `AGENTS.md` mirrors it.

## The project in one paragraph

MeedyaManager is a media file manager and automatic organiser for Windows, macOS and Linux. The
engine is written in Rust (folders under `crates/`); the macOS app in Swift, the Windows app in
C#, and the Linux app in Rust with GTK4. It reads and writes the tags inside music and video
files, renames and moves them according to user-written rules, and watches folders to do this
automatically. Shared code for every "Meedya" application lives upstream in
**MeedyaSuite-core** — check there before saying something is missing.

## Codex's usual role here

Codex is the **independent reviewer** for work that Claude Code builds, reviewing round after
round until nothing is found. When Claude is unavailable, Codex may build instead, and then
Claude reviews Codex's work. Either way, record it in the handoff.

**Practical note:** run `codex exec` with `< /dev/null`, or it waits forever for extra input.
