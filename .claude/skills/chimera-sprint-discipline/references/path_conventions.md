# Path Conventions

Single source for the file-path and case rules used across phase_audit,
batch_planning, and phase_review. Cite this file from process steps instead of
re-explaining paths inline.

## Artifacts per phase {X.Y}

| Artifact | Path | Case |
|---|---|---|
| Phase doc (sparse manifest) | `docs/phases/phase-{X.Y}.md` | lowercase `phase-` |
| Phase audit | `docs/audits/{prerequisite-sprint-id}.md` | sprint id (see below) |
| Batch plan | `docs/plans/Phase-{X.Y}-batch.md` | capital `Phase-` |
| Per-sprint summaries | `docs/sprints/phase-{X.Y}/{sprint-id}.md` | lowercase `phase-` |

## Audit naming
The audit is named for the sprint that authors it — typically the phase's first
read-only sprint. General pattern: `docs/audits/{phase}.0.md`
(e.g. `docs/audits/FC.0.md` for Phase III.C, `docs/audits/V.A.0.md` for Phase V.A).
There is no separate phase-level audit artifact — the prerequisite sprint IS the audit.

## Case asymmetry (match disk reality)
Phase DOCS are lowercase (`docs/phases/phase-{X.Y}.md`) but phase PLANS are
capital-P (`docs/plans/Phase-{X.Y}-batch.md`). This is deliberate; match it exactly.
