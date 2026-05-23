# Batch Execution Process

<key_insight>
Batch execution runs pre-approved sprints serially in one session.
Each sprint commits independently. A red-line violation HALTS the batch.
Trust comes from per-sprint commit isolation, not from per-sprint approval.
</key_insight>

## Hard Preconditions
1. Batch plan exists at docs/phases/phase-{X}.md or referenced batch document
2. User has explicitly invoked "execute batch" / "run FC.N..M"
3. Python env path declared in CLAUDE.md (for verification commands)

## Per-Sprint Loop

For each sprint in batch order:

<step n="1">
Read sprint definition from batch plan. Confirm files-in-scope, red lines, acceptance.
</step>

<step n="2">
Read target files IN FULL. Read associated tests.
Grep call sites of any function planned to modify.
</step>

<step n="3">
Apply edits via Edit/MultiEdit. Never reconstruct via Write.
</step>

<step n="4">
Spawn subagent (Haiku) to run check_taste.sh + pytest. Receive structured summary.
</step>

<step n="5">
If self-check reveals red-line violation:
  - HALT entire batch
  - Output violation summary + which sprint failed
  - Do NOT proceed to next sprint
</step>

<step n="6">
If self-check passes:
  - Write sprint summary to `docs/sprints/phase-{X.Y}/{sprint-id}.md`
    using assets/modification-summary-template.md
  - Stage all sprint files + the summary doc
  - Commit with Tier-2 message:
    - Subject: `feat({scope}): {sprint-id} — {one_line}`
    - Body: 3-5 line summary + `Refs: docs/sprints/phase-{X.Y}/{sprint-id}.md`
  - Proceed to next sprint
</step>

## After Batch

<step n="7">
After all sprints complete (or batch halted), output final summary:
- Sprints completed (list)
- Sprints not run (if halted)
- Aggregate file changes
- Aggregate verification status
</step>

<step n="8">
Hand off to chimera-sprint-discipline phase_review mode for sealing.
</step>

## Success Criteria
- [ ] Each completed sprint has its own commit
- [ ] No commit contains changes from multiple sprints
- [ ] Verification ran for every sprint
- [ ] Halt on red line — never silently bypass
- [ ] Final summary delivered
