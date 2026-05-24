# Batch Execution Process

<key_insight>
Batch execution runs pre-approved sprints serially in one session.
Each sprint commits independently. A red-line violation HALTS the batch.
Trust comes from per-sprint commit isolation, not from per-sprint approval.
</key_insight>

## Hard Preconditions
1. Batch plan exists at `docs/plans/{phase}-batch.md` (e.g., `docs/plans/Phase-III.C-batch.md`)
2. User has explicitly invoked "execute batch" / "run FC.N..M"
3. Python env path declared in CLAUDE.md (for verification commands)

## Per-Sprint Loop

For each sprint in batch order:

<step n="0">
Read `docs/phases/phase-{X.Y}/_progress.md` if exists.

If exists: identify last completed sprint and assumed resumption point.
Verify predecessor assumptions hold via Git log + targeted Greps.
If holds → resume from next sprint. If broken → halt and surface to user.

If not exists: this is a fresh batch start. Create the file with header.
</step>


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
  - **Append progress entry to `docs/phases/phase-{X.Y}/_progress.md`**
    (one line: status + commit hash + accepted partials)
  - Stage all sprint files + summary + progress entry
  - Commit with Tier-2 message
  - Proceed to next sprint
</step>

<step n="N">
Session boundary trigger: when context usage exceeds ~70% OR user explicitly
invokes "session pause":

  1. Append session-boundary entry to `docs/phases/phase-{X.Y}/_progress.md`:
     - Sprints completed this session
     - Cumulative accepted partials this session
     - Process drift observations (e.g., hitched commits)
     - Next session resumption point + predecessor assumptions to verify
  2. Output brief summary in conversation
  3. Hand off — do NOT continue past boundary in same session

The next session bootstraps with `/clear`-equivalent (new session) and
reads progress file at step 0 above.
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
