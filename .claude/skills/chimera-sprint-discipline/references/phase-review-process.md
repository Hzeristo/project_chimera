# Phase Review Process

<key_insight>
The question is not "did all sprints commit cleanly?" — it is "did the phase
resolve the frictions that drove it, were red lines held, and is every Partial
correctly categorized?"

A green commit log with violated red lines is a failed review.
A halted batch with documented Accepted Partials may still be a successful review.
</key_insight>

## Hard Preconditions

1. Phase audit exists at the phase audit path (see references/path_conventions.md)
2. Batch plan exists at the batch-plan path (capital-P; see references/path_conventions.md)
3. Batch execution has completed (or halted) — at least one sprint commit exists
4. User explicitly invoked review (not auto-triggered after batch_execution)

If any precondition fails, STOP. Output diagnosis. Do not proceed.

## Steps

<step n="0">
Read `docs/sprints/phase-{X.Y}/*.md` summaries to recover batch history
across the session(s). These per-sprint summaries are the execution record —
the source of truth for what was done (see ../../_shared/doc_folders.md).
Extract:
- Per-sprint completion order + commits
- Accepted Partials accumulated
- Process drift observations
- Session boundaries (for cross-session consistency check)

Fallback — if `docs/sprints/phase-{X.Y}/` does not exist or is empty (e.g. a
phase sealed before the per-sprint-summary protocol existed):
- Reconstruct batch history from the sprint list in `docs/phases/phase-{X.Y}.md`
  (intent), cross-checked against `git log`.
- Mark the verdict "reconstructed" — the evidence base is weaker, so say so
  explicitly rather than presenting it as a full record.
</step>


<step n="1">
Identify scope: all sprints in the phase batch, completed or not.

```
PowerShell("git log --oneline -50 | Select-String -Pattern 'phase-{X}' -CaseSensitive:$false")
PowerShell("git log {first_sprint_commit}..HEAD --stat")
```

For halted batch: identify which sprints completed vs which never ran.
</step>

<step n="1+">
For each completed sprint, also Read `docs/sprints/phase-{X.Y}/{sprint-id}.md`
to recover per-sprint verification context the commit body could not hold.
</step>

<step n="2">
Read phase audit to recover original questions and cross-findings.
Read batch plan to recover declared red lines and acceptance criteria per sprint.

```
Read("docs/audits/{prerequisite-sprint-id}.md")
Read("docs/plans/Phase-{X.Y}-batch.md")
```
</step>

<step n="3">
For each completed sprint, verify acceptance criteria via Grep / pytest output / file inspection.

Run red-line scans DIRECTLY in the main session — do NOT delegate. At seal time
correctness outranks speed, and a delegated prose result is not authoritative.
Grep each forbidden pattern declared in `docs/phases/phase-{X.Y}.md` across src/
yourself:

```
Grep(pattern="except BaseException", path="src", output_mode="content")
Grep(pattern="TOOL_REGISTRY\\[", path="src", output_mode="content")
# ...one Grep per declared red line. 0 hits = Held; any hit = Violated (file:line).
```

If a scan is genuinely too broad to run inline, a Haiku scout MAY pre-collect raw
match lines, but the main session MUST re-Grep each flagged file to confirm before
recording Held/Violated. The scout's summary is never the verdict.
</step>

<step n="4">
For each finding (per sprint and phase-wide), categorize per partial-triage rules:

| Category | Definition | Action |
|---|---|---|
| **Pass** | Acceptance fully satisfied | None |
| **Accepted Partial** | Trade-off declared upfront in batch plan, OR confirmed acceptable now | Append to `docs/ACCEPTED_PARTIALS.md` |
| **Technical Debt** | Deficiency discovered during review, not blocking | Append to `docs/TECHNICAL_DEBT.md` |
| **Fail** | Red line violated OR existing behavior broken | Block sealing, propose minimal patch (<30 lines) |

Triage decision tree:
1. Was the trade-off declared upfront in batch plan? → Yes: Accepted Partial.
2. Does it violate a red line? → Yes: Fail.
3. Does it break existing behavior? → Yes: Fail.
4. Will future maintainer understand why we accepted it? → Yes: Accepted Partial. → No: Technical Debt.
</step>

<step n="5">
Verify hard sealing conditions from `docs/phases/phase-{X.Y}.md`. These are the
phase-level acceptance criteria, distinct from per-sprint criteria.

For each sealing condition: explicit Pass/Fail with file:line or test output.
</step>

<step n="6">
Verify each driving friction from phase doc has been addressed:

```
Glob("docs/logs/friction-*.md")
```

For each friction listed in phase doc:
- Has the entry's status moved from OPEN/SCHEDULED to RESOLVED?
- If not, is there a documented reason (deferred, obsoleted, partial)?
</step>

<step n="7">
Emit verdict using `assets/phase-review-verdict-template.md`.

Sealing decision:
- ✅ **Sealed**: All Pass or only Accepted Partials. Hard sealing conditions met. Frictions resolved or deferred with reason.
- ⚠️ **Functionally Sealed**: Pass + Accepted Partials + N Technical Debt items filed.
- ❌ **NOT Sealed**: Any Fail, OR hard sealing condition not met, OR red line violated.
</step>

<step n="8">
Apply state file updates by category:

**Auto-apply (mechanical, no decision):**
- ACCEPTED_PARTIALS.md: append new partials with full context
- TECHNICAL_DEBT.md: append new DEBT-{id} entries
- friction-*.md: flip status OPEN/SCHEDULED → RESOLVED for entries this phase resolved

**Propose diff for user approval (decision-bearing):**
- ROADMAP.md phase status (Active → Sealed / Functionally Sealed)
- friction-*.md status changes that aren't direct phase-resolutions
  (e.g., reclassify OPEN → WONTFIX based on new evidence)
- Any state file change that conflicts with existing content

For auto-apply category:
1. Use Edit / PowerShell Out-File -Append per the file's append-only or mutate semantics
2. Verify by Read after write — confirm entry visible. If the Read does NOT show
   the entry, HALT and surface to user; do not stage a partial or failed write.
3. Stage but DO NOT commit (user owns commit message for state changes)

For propose-diff category:
- Output unified diff
- Wait for user approval
- Only then apply via Edit
</step>

<step n="N+1">
After phase_review verdict accepted by user:
- Verify all per-sprint summaries exist in docs/sprints/phase-{X.Y}/
</step>


## Examples

For a worked bad/good verdict, see the appendix of
`assets/phase-review-verdict-template.md`.

## Success Criteria
- [ ] Every sprint in batch has explicit verdict + evidence
- [ ] Every Partial categorized with reason
- [ ] Every red line verified via Grep/PowerShell
- [ ] Hard sealing conditions checked individually
- [ ] Driving frictions status verified
- [ ] Sealing decision unambiguous (Sealed / Functionally Sealed / NOT Sealed)
- [ ] Proposed diffs for state files output as unified diff
- [ ] No code modifications attempted
- [ ] State files written per spec, all staged, zero commits
