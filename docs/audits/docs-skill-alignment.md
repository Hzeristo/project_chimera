# docs/ ↔ Skill Path Alignment Audit

**Date:** 2026-06-27
**Scope:** All 5 chimera skills (`.claude/skills/*`, incl. `_shared/`) vs. the
`docs/` tree (`phases/ plans/ sprints/ audits/ staging/ incidents/ logs/
architecture/` + root state files).
**Mandate:** Catalog only — flag every place a skill references a path/file that
doesn't exist or a pattern that conflicts with on-disk reality. **No fixes
applied.** Positive (correct) findings included so they aren't "fixed" by mistake.

**Legend (skill file shorthands):**
SD = sprint-discipline/SKILL.md · PR = …/references/phase-review-process.md ·
PA = …/phase-audit-process.md · BP = …/batch-planning-process.md ·
PC = …/path_conventions.md · VT = …/assets/phase-review-verdict-template.md ·
PAT = …/assets/phase-audit-template.md · BPT = …/assets/batch-plan-template.md ·
CT = code-taste/SKILL.md · BE = code-taste/references/batch_execution_process.md ·
INC = _shared/incident_protocol.md · CP = core-philosophy/SKILL.md ·
DV = dependency-veto/SKILL.md.

---

## Q1 — What paths do skills expect?

Grep of every skill `.md` for `docs/…` references, grouped by target.

| # | Expected path / pattern | Token(s) | Case convention | Cited at (file:line) |
|---|---|---|---|---|
| 1 | `docs/ROADMAP.md` | — | exact | SD:23,64; VT:86; BP:13 |
| 2 | `docs/ACCEPTED_PARTIALS.md` | — | exact | SD:12,25,59; PR:85; VT:77 |
| 3 | `docs/TECHNICAL_DEBT.md` | — | exact | SD:13,26,60; PR:86; VT:79 |
| 4 | `docs/plunder_list.md` | — | exact | CP:36,66; DV:52 |
| 5 | `docs/logs/friction-*.md` | date in name | `friction-` lc | SD:24; PR:107; BP:13; VT:147-148 |
| 6 | `docs/incidents/{YYYY-MM-DD}-{slug}.md` | date+slug | lc | INC:15; SD:110; CT:97 |
| 7 | `docs/phases/phase-{X.Y}.md` | `{X.Y}` | **lowercase `phase-`** | PA:10; PC:11,23; PR:65,97; BPT:5 |
| 8 | `docs/plans/Phase-{X.Y}-batch.md` | `{X.Y}` | **capital `Phase-`** | PC:13; BPT:3; BE:10(e.g.) |
| 8b | `docs/plans/{phase}-batch.md` | `{phase}` | **token carries no `Phase-` prefix** | PR:56; BE:10; VT:4; BPT:101 |
| 9 | `docs/audits/{prerequisite-sprint-id}.md` (general `{phase}.0.md`) | `{prereq-sprint-id}` / `{phase}` | sprint-id | PC:12,18-19; PA:59; PR:55; VT:3; PAT:4; BPT:4 |
| 10 | `docs/sprints/phase-{X.Y}/{sprint-id}.md` (DIR per phase) | `{X.Y}`,`{sprint-id}` | lowercase `phase-` | PR:24,46,152; PC:14; BE:21,70,85 |
| 11 | `docs/architecture/{PROMPT_MIDDLEWARE,TOOL_PROTOCOL,INTENT_AND_DEGRADATION,TASK_PROGRESS_SYSTEM}.md` | — | **lowercase `architecture/`** | CT:20-23 |

---

## Q2 — What's actually on disk?

```
docs/ROADMAP.md  ACCEPTED_PARTIALS.md  TECHNICAL_DEBT.md  plunder_list.md
docs/logs/        friction-260426 / -260506 / -260518 / -260523 / -260526 / -260611 .md
docs/incidents/   2026-06-11-call-fence-parse-failure.md  (+4 dated-slug files)
docs/phases/      phase-III.C  phase-III.E  phase-III.F  phase-IV.A  phase-V.A  phase-EXT  (.md, lowercase)
docs/plans/       Phase-III.F-batch.md  Phase-IV.A-batch.md  Phase-V.A-batch.md  (capital Phase-)
docs/audits/      FC.0  III.E.0  IV.A.0  V.A.0  (.md)  + V.A.2b / FC.5-verify / *-e2e / topic audits
docs/sprints/     phase-EXT/(6)  phase-III.C/(8)  phase-III.E/(0, empty)  phase-IV.A/(1)
docs/architecture/ PROMPT_MIDDLEWARE TOOL_PROTOCOL INTENT_AND_DEGRADATION TASK_PROGRESS_SYSTEM  (+7 more, lowercase dir)
docs/staging/     (empty)
docs/FINAL_CONTRACT/ V.A-final-contract.md   ← not referenced by any skill
```

---

## Q3a — Positive findings (correct; DO NOT "fix")

| # | Expected | Disk reality | Verdict |
|---|---|---|---|
| 1 | `docs/ROADMAP.md` | present | ✅ exact |
| 2 | `docs/ACCEPTED_PARTIALS.md` | present | ✅ exact |
| 3 | `docs/TECHNICAL_DEBT.md` | present | ✅ exact |
| 4 | `docs/plunder_list.md` | present | ✅ exact |
| 5 | `docs/logs/friction-*.md` | 6 files `friction-26MMDD.md`; VT's `friction-260506.md` example exists | ✅ pattern + example real |
| 6 | `docs/incidents/{date}-{slug}.md` | 5 files match the date-slug shape exactly | ✅ pattern |
| 7 | `docs/phases/phase-{X.Y}.md` **lowercase** | `phase-III.C.md` … `phase-V.A.md`, `phase-EXT.md` — all lowercase | ✅ **case matches** |
| 8 | `docs/plans/Phase-{X.Y}-batch.md` **capital** | `Phase-III.F/IV.A/V.A-batch.md` — all capital-P | ✅ **case matches** (canonical refs PC:13, BPT:3) |
| 9 | `docs/audits/{phase}.0.md` | `FC.0` (III.C), `III.E.0`, `IV.A.0`, `V.A.0` — incl. the documented FC.0-for-III.C special case (PC:18-19) | ✅ pattern |
| 10 | `docs/sprints/phase-{X.Y}/` DIR holding `{sprint-id}.md` | dirs are real dirs (not files) holding `FC.1.md`, `EXT.1.md`, … | ✅ **structure matches** (for phases that have it — see Q3c) |
| 11 | `docs/architecture/*.md` **lowercase** | dir is lowercase `architecture/`; all 4 named files present | ✅ **case + files match** |

---

## Q3b — Mismatches (skill text vs. disk)

| ID | Class | Skill ref (file:line) | Expected | Actual state | Severity |
|---|---|---|---|---|---|
| **M1** | **PATTERN** | PR:56 `Read("docs/plans/{phase}-batch.md")` (also BE:10, VT:4, BPT:101) | If `{phase}` is naively expanded to `III.F`, yields `docs/plans/III.F-batch.md` | Disk uses **`Phase-III.F-batch.md`** (literal `Phase-` prefix, capital P). The `{phase}` token does NOT carry the prefix, so the literal expansion misses. PR:56 is the worst case — bare, no inline `e.g.` to correct it. | **P1** — a wrong `Read` → error/Glob fallback. Mitigated (not eliminated) by PC:13/24. |
| **M2** | **MISSING (example)** | PC:13, BPT:3, BE:10, VT:4, PAT/BPT audit lines (all as "e.g. `docs/plans/Phase-III.C-batch.md`") | The most-cited example file | **`Phase-III.C-batch.md` does not exist on disk** (III.C predates the `plans/` convention — III.C has a sprint dir + FC.0 audit but no saved batch plan). | **P2** — illustrative only, but the canonical example points at the one phase whose plan was never saved → confusing. |
| **M3** | **CASE (adjacent)** | Not a skill — `CLAUDE.md:37` "See `docs/ARCHITECTURE/`" (user's scope also wrote `ARCHITECTURE/`) | uppercase `ARCHITECTURE/` | Disk + the skill that actually reads it (CT:20-23) use **lowercase `architecture/`**. Harmless on Windows (case-insensitive FS); would break a case-sensitive checkout. The *skills* are correct; only CLAUDE.md and the mental model are upper. | **P2** — cosmetic, non-skill. |

No **CASE** mismatches inside skills (phases lowercase ✓, plans capital ✓,
architecture lowercase ✓). No **STRUCTURE** inversions (every `phase-{X.Y}/`
that exists is a directory, never a flat file). No true **MISSING** for any
required (non-example) skill path.

---

## Q3c — Instance-coverage gaps (per-phase artifacts the skill *preconditions* expect)

These are not skill-text bugs (the patterns are sound and match disk where the
artifacts exist). They are **state gaps**: for specific phases, an artifact a
skill workflow requires is absent, so that workflow would fail/short-circuit for
that phase. Surfaced because the mandate is "a path a skill references that
doesn't exist" — at the instance level, these qualify.

Per-phase matrix (✓ present / ✗ absent):

| Phase | `phases/phase-X.md` | `plans/Phase-X-batch.md` | `audits/{…}.0.md` | `sprints/phase-X/` | Notes |
|---|---|---|---|---|---|
| III.C | ✓ | ✗ | ✓ FC.0 | ✓ (8) | complete except batch plan never saved (M2) |
| III.E | ✓ | ✗ | ✓ III.E.0 | ✓ but **empty (0)** | audited, never planned/executed (or summaries not kept) |
| III.F | ✓ | ✓ | ✗ III.F.0 | ✗ | has plan but **no audit** (inverts "audit before plan", SD core_principle #2) and no sprint dir |
| IV.A | ✓ | ✓ | ✓ IV.A.0 | ✓ (1) | **only fully-complete phase** |
| V.A | ✓ | ✓ | ✓ V.A.0 | ✗ | **sealed 2026-06-16 but has no `sprints/phase-V.A/`** |
| EXT | ✓ | ✗ | ✗ | ✓ (6) | executed sprints but no audit + no batch plan |

| ID | Affected skill workflow | Gap | Severity |
|---|---|---|---|
| **C1** | phase_review of V.A — PR:24,46,152 read `docs/sprints/phase-V.A/*.md` as the batch-history source of truth | dir absent for the **last sealed phase**; a re-review or audit-trace of V.A would fail step 0 | **P1** (V.A already sealed, so not blocking forward work — but any V.A re-review is broken) |
| **C2** | batch_planning / phase_review of III.F — both require `docs/audits/III.F.0.md` (BP precondition 1, PR precondition 1) | III.F has a batch plan but **no `.0` audit** → planning ran (or plan exists) without the required audit artifact | **P1** if III.F is active/next; **P2** if queued/future |
| **C3** | phase_review of III.E — PR step 0 reads `sprints/phase-III.E/*.md` | dir exists but **empty** → step 0 yields nothing | **P2** (III.E appears unexecuted) |
| **C4** | batch_planning of EXT — needs `docs/audits/EXT.0.md` | EXT has 6 sprint summaries but **no audit and no batch plan** → preconditions were bypassed | **P2** (EXT executed; retroactive only) |

> These four are **process/state observations**, not skill-file defects. The
> skill text is internally consistent and matches disk for IV.A (the complete
> case). Whether to backfill is a user call, not a rename or a skill edit.

---

## Fix plan

Split by remedy. **Nothing here is applied** — this is the menu.

### A. Fix by updating skill references (disk is canonical; text is ambiguous)
1. **M1 (P1):** Replace the bare `{phase}-batch.md` token with the explicit
   `Phase-{X.Y}-batch.md` form at **PR:56** (highest risk, no inline e.g.), and
   for consistency at BE:10, VT:4, BPT:101. Rationale: PC:13 already declares
   capital-`Phase-` as canonical and disk agrees — the `{phase}` token is the
   only thing implying otherwise. One-line edits; removes the wrong-Read path.
2. **M2 (P2):** Switch the canonical example from `Phase-III.C-batch.md` to a
   phase whose full artifact set exists — **IV.A** (the only doc+plan+audit+
   sprintdir-complete phase). Update PC:13, BPT:3, BE:10, VT:4 examples. Keeps
   examples copy-paste-real.

### B. Fix by renaming files on disk
*None.* Every case/structure convention the skills declare already matches disk.
No rename would improve alignment (and renaming `phase-`/`Phase-` would break the
deliberate asymmetry PC documents).

### C. Neither — user state decisions (backfill or accept)
3. **C1 (P1):** Decide whether to backfill `docs/sprints/phase-V.A/` summaries
   (needed only if V.A is ever re-reviewed) or accept that the per-sprint-summary
   protocol postdates V.A and document V.A as exempt.
4. **C2–C4 (P1/P2):** Confirm III.F/III.E/EXT status. If III.F is "next", create
   its `III.F.0` audit before planning (satisfies SD core_principle #2). If
   III.E/EXT are historical, mark them so the missing audits aren't read as drift.

### D. Adjacent (outside skill scope, needs approval)
5. **M3 (P2):** Lowercase `CLAUDE.md:37` `docs/ARCHITECTURE/` → `docs/architecture/`
   to match disk and the skill. CLAUDE.md edit requires user approval.

---

## Bottom line

The skill path **conventions are sound and match disk** — phases lowercase,
plans capital-`Phase-`, architecture lowercase, audits/incidents/sprints
patterns all confirmed against real files (Q3a). The only **skill-text** issue is
**M1**: a `{phase}-batch.md` token (notably the bare one at PR:56) that doesn't
carry the `Phase-` prefix disk requires — P1, fix in skill text, do not rename.
**M2** is a stale canonical example. Everything else (C1–C4) is per-phase state
coverage — most visibly **V.A is sealed yet has no `sprints/phase-V.A/` dir** —
which is a process/backfill decision, not a path-convention defect.

*Audit only — no files renamed, no skill references edited.*
