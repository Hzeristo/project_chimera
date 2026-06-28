# Doc Folders — Source of Truth & R/W Rules (shared)

Single source for what each `docs/` folder means and which skill may read or
write it. Both process skills (chimera-sprint-discipline, chimera-code-taste)
point here. The conceptual model is mirrored in CLAUDE.md ("Source of truth");
exact filename/case forms live in
chimera-sprint-discipline/references/path_conventions.md.

## A phase's record = sprints + audits

Three folders answer three different questions. None substitutes for another.

| Folder | Question | Role |
|---|---|---|
| `docs/phases/phase-{X.Y}.md` | what to do | INTENT (sparse manifest) |
| `docs/sprints/phase-{X.Y}/{sprint-id}.md` | what was done | EXECUTION record |
| `docs/audits/{prereq-sprint-id}.md` | right or wrong | EVIDENCE — usually the pre-sprint prerequisite audit; exceptions exist (review / debt) |
| `docs/plans/Phase-{X.Y}-batch.md` | how (derived from the audit) | PLAN |

## Read / write authority

| Folder | chimera-sprint-discipline | chimera-code-taste |
|---|---|---|
| `phases/` | READ only — audit/plan/review against intent | READ only — red lines, acceptance |
| `audits/` | **WRITE** (phase_audit authors `{phase}.0.md`) + READ | READ (optional context) |
| `plans/`  | **WRITE** (batch_planning) + READ (review) | READ — sprint scope |
| `sprints/`| READ only — phase_review batch history | **WRITE** — per-sprint summary at each commit |

Neither skill writes `phases/`; it is user-authored intent. Source code and
architecture docs are written only by code-taste, within sprint scope. State
files (ROADMAP / ACCEPTED_PARTIALS / TECHNICAL_DEBT / friction logs) follow
chimera-sprint-discipline's own `state_write_authority`, not this table.

## Autonomy
Each skill writes its own output artifact **autonomously** — no per-write
approval, because the artifact IS the deliverable: batch_planning → `plans/`,
batch_execution → `sprints/`, phase_audit → `audits/`. `phases/` is the
exception — human-authored intent, only humans write it. Decision-bearing state
edits (ROADMAP, friction reclassification) still propose-diff.

## Why the separation matters
The workflows depend on these staying distinct: planning reads the audit (the
**evidence** base), not the phase intent; review reconstructs history from the
**execution** record, not the plan. Conflating them — e.g. treating the phase
doc as the execution record — makes a review claim things happened that were only
ever proposed.

## Fallback when the execution record is missing
If `docs/sprints/phase-{X.Y}/` does not exist or is empty (e.g. a phase sealed
before the per-sprint-summary protocol existed), reconstruct batch history from
the sprint list in `docs/phases/phase-{X.Y}.md` (intent) cross-checked against
`git log`, and mark the review "reconstructed". The evidence base is then weaker
— say so, rather than presenting a reconstructed history as a full record.
