---
name: chimera-commit-style
description: Git commit message style for Chimera. Activate when drafting commit messages or discussing git hygiene. Three-tier message length based on change significance. Personal project conventions, not team OSS conventions.
allowed-tools:
  - Read
  - Grep
  - Glob
  - Edit
---

# Chimera Commit Style

## Three-Tier Model

Chimera is a personal project. The audience of commit messages is future-you, not a distributed team. Message length should be proportional to future query probability.

### Tier 1: Single Line (Default, ~80% of commits)

For typo fixes, small refactors, doc touch-ups, incremental improvements.

```
fix(oligo): strip reasoning tags in stream chunks
refactor(vault): extract path resolution to helper
docs: update PROMPT_MIDDLEWARE.md component list
chore: bump test fixtures
```

### Tier 2: Multi-Line with Body (~15% of commits)

For sprint-level changes spanning 2-3 related files, or when "why" matters more than "what".

```
feat(agent): tool result reflection hints

- structured <tool_result status=.../> wrapping
- EMPTY_RESULT fallback suggestion
- no hint when all tools succeed

Refs: friction E3 partial.
```

### Tier 3: Full Phase Closure (~5% of commits)

For phase / milestone sealing. Future-you will query these when writing technical reports.

```
feat(oligo): Phase III.B.3 — intent recognition & graceful degradation

- prompt: tri-tier tool list rendering (verbose/compact/micro)
- prompt: ROUTER_INTRO zero-arg guidance
- agent: typed <tool_result status=.../> with reflection hints
- sse: bb-tool-start / bb-tool-done per-call telemetry
- ui: ActiveToolStrip with 0.1s resolution timer
- prompt_composer: xml_structured renderer
- docs: INTENT_AND_DEGRADATION.md
- tests: unit + smoke scripts

Accepted partials:
- Tool list compresses under length budget
- Zero-arg emission not asserted against live LLM
- DENIED-only batches emit no telemetry

Refs: friction E1, E2 (partial), E3 (partial).
Phase III.A closed. Phase III.B.3 sealed. Phase III.C next.
```

## Scope Tags

Use one of these scope tags:

- `oligo` — Oligo agent (Python)
- `astrocyte` — Astrocyte frontend (Svelte/Tauri/Rust)
- `crucible` — shared core (schemas, config, platform, ports, services)
- `vault` — Obsidian vault adapters
- `tools` — oligo tools
- `docs` — documentation only
- `tests` — test-only changes
- `chore` — build / config / dependency updates
- `deps` — dependency additions (requires justification, see `chimera-dependency-veto`)

## Type Prefixes

Conventional commits style:
- `feat` — new capability
- `fix` — bug fix
- `refactor` — no behavior change
- `docs` — docs only
- `test` — test only
- `chore` — maintenance
- `perf` — performance
- `style` — formatting
- `revert` — revert previous commit

## Never Do

- **Never** rewrite old commit history for message aesthetics. Git history is an append-only audit log.
- **Never** force push to branches after they're shared, even in personal repos — you might regret it.
- **Never** write a tier-3 message for a tier-1 change. That's ceremony theater.
- **Never** write a tier-1 message for a phase closure. Future-you will curse you.

## The Three-Months-Later Test

For each commit, ask:

> "Three months from now, if I only see this commit message, do I need to know more than what's here?"

- Typo fix → no → Tier 1
- Sprint closure → maybe → Tier 2
- Phase closure → yes, will quote in PhD report → Tier 3

## Partial Acceptance in Commit Body

When sealing a sprint with accepted partials, list them explicitly in commit body under `Accepted partials:`. This creates an audit trail of intentional trade-offs separate from technical debt.

## References

Link to frictions when applicable:
- `Refs: friction E1, E3`
- `Refs: audit report 2026-04-25`

This lets future-you trace commits back to motivation.