# Rules for every project on this computer

> **(C) 2025-2026 MWBM Partners Ltd**
>
> The owner asked on 2026-09-23 for these rules to apply to **every** project, not just
> MeedyaManager. A cloud session cannot reach the owner's own computer, so the text is kept
> here, ready to copy.
>
> **To apply it:** paste everything below the line into
> - `~/.claude/CLAUDE.md` — read by Claude Code in every project, and
> - `~/.codex/AGENTS.md` — read by Codex in every project.
>
> Add it to what is already there; do not replace existing rules.

---

## Write in plain English

When giving feedback or explaining anything, do not use technical jargon — it can confuse
even technically skilled people. Use plain, everyday English. Where a technical term cannot
be avoided, explain it in ordinary words.

## Keep a handoff document, and keep it current

Each project keeps a handoff document (for example `.claude/HANDOFF.md`) that records where
the work really stands. Update it **as you go**, not only at the end, so the work can be
picked up at any time after an interruption. Record only what has been checked. Temporary work
folders, scratch files and unpushed commits do not survive a new session, so commit finished
work and — following each project's own rule on pushing — say plainly what is still waiting
to be pushed.

## When an AI service is unavailable — fall back, then switch back

If the main AI service for a project, or one of its helpers, becomes unavailable or runs out
of usage credits, hand the work to another suitable AI service or tool — provided that can be
done without losing context or progress. This applies to whichever services are in use; it is
not tied to particular products.

- Switch back to the main service as often as possible.
- Once the main service is available again, have it do a **full review** of everything done
  while it was away.
- This is reasonably safe because work is also cross-checked by a different AI system, which
  should catch differences in how each one works.
- It makes an up-to-the-minute handoff document essential: it is the only thing the stand-in
  can rely on. Record each switch there — who stood in, for what, and what review is owed.

## Check work with a different AI system

Work planned and built with one AI system (for example Claude Code) is reviewed with another
(for example Codex), and the other way round. Fix what the review finds and review again until
a review finds nothing. Aim: Get It Right First Time.
