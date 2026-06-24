# Batch Planning Process

<key_insight>
Batch planning produces a sprint sequence the user reads asynchronously,
not isolated sprint prompts awaiting individual approval.
The risk: planning sprint N based on assumed outcomes of sprint M (M<N).
The mitigation: sprint dependencies explicit, predecessor assumptions named.
</key_insight>

## Hard Preconditions
1. Phase audit complete at docs/audits/{prerequisite-sprint-id}.md (general pattern `docs/audits/{phase}.0.md` — e.g. `docs/audits/FC.0.md`, `docs/audits/V.A.0.md`)
2. Phase doc declares sprint name list (sparse manifest)
3. ROADMAP.md + docs/logs/friction-*.md confirm phase is current active

## Steps

<step n="1">
Read phase audit IN FULL. Identify confirmed facts vs open questions.
</step>

<step n="2">
For each sprint name in phase doc, derive:
- Single objective (one sentence, verb-led)
- Driving friction reference
- File scope (audit-derived, not phase-doc-derived)
- Acceptance criteria (audit-derived)
- Red lines (≥3, including phase-wide red lines + sprint-specific)
- Risk level: 🟢 LOW / 🟡 MED / 🔴 HIGH
</step>

<step n="3">
Identify split opportunities. If a sprint exceeds ≤3 files / ≤50 lines,
propose split (e.g., FC.2 → FC.2a/FC.2b). Splits are allowed; expansion is not.
</step>

<step n="4">
Identify any sprint that audit revealed is "already done" or "test-only".
Mark these and reduce their scope accordingly.
</step>

<step n="5">
Output sprint sequence using assets/batch-plan-template.md.
Write to `docs/plans/{phase}-batch.md`, where `{phase}` is capital-P on disk:
`Phase-{X.Y}-batch.md` (e.g. `docs/plans/Phase-III.C-batch.md`, `docs/plans/Phase-V.A-batch.md`).
Note the case asymmetry: phase DOCS are lowercase (`docs/phases/phase-{X.Y}.md`)
but phase PLANS are capital-P (`docs/plans/Phase-{X.Y}-batch.md`). Match disk reality.
The sequence is a single document — user approves whole or rejects whole.

Note: the batch plan is a distinct artifact from the phase doc.
- Phase doc (`docs/phases/phase-{X.Y}.md`): sparse manifest, sprint names + one-line goals.
- Batch plan (`docs/plans/{phase}-batch.md`): detailed task scope, file lists, red lines.
</step>

## Success Criteria
- [ ] Every sprint has audit file:line citations
- [ ] Predecessor assumptions explicit
- [ ] Risk levels assigned
- [ ] Split opportunities identified
- [ ] No code modification
