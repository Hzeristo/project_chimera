# Phase Review Process

<key_insight>
The question is not "did all sprints commit cleanly?" — it is "did the phase
resolve the frictions that drove it, were red lines held, and is every Partial
correctly categorized?"

A green commit log with violated red lines is a failed review.
A halted batch with documented Accepted Partials may still be a successful review.
</key_insight>

## Hard Preconditions

1. Phase audit exists at `docs/audits/{prerequisite-sprint-id}.md` (e.g., `docs/audits/FC.0.md`)
2. Batch plan exists at `docs/plans/{phase}-batch.md` (e.g., `docs/plans/Phase-III.C-batch.md`)
3. Batch execution has completed (or halted) — at least one sprint commit exists
4. User explicitly invoked review (not auto-triggered after batch_execution)

If any precondition fails, STOP. Output diagnosis. Do not proceed.

## Steps

<step n="0">
Read `docs/phases/phase-{X.Y}/_progress.md` to recover full session history
across the batch. Extract:
- Per-sprint completion order + commits
- Accepted Partials accumulated
- Process drift observations
- Session boundaries (for cross-session consistency check)
</step>


<step n="1">
Identify scope: all sprints in the phase batch, completed or not.

```
Bash("git log --oneline -50 | grep -i 'phase-{X}'")
Bash("git log {first_sprint_commit}..HEAD --stat")
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
Read("docs/plans/{phase}-batch.md")
```
</step>

<step n="3">
For each completed sprint, verify acceptance criteria via Grep / pytest output / file inspection.
Spawn subagent (Haiku) for repo-wide red-line scans:

```
Task(
  subagent_type="general-purpose",
  prompt="Grep for forbidden patterns across src/: 'except BaseException', 'TOOL_REGISTRY[', any other red lines from phase-{X.Y}.md. Return file:line of any matches."
)
```
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
Output proposed diffs for state files:
- `docs/ROADMAP.md`: phase status update (Active → Sealed / Functionally Sealed)
- `docs/ACCEPTED_PARTIALS.md`: new entries to append
- `docs/TECHNICAL_DEBT.md`: new DEBT-{id} entries to append
- `docs/logs/friction-*.md`: status updates for resolved frictions

Output as unified diff blocks. Do NOT apply directly. User reviews and applies.
</step>

<step n="N+1">
After phase_review verdict accepted by user:
- Verify all per-sprint summaries exist in docs/sprints/phase-{X.Y}/
- Delete docs/phases/phase-{X.Y}/_progress.md (transient artifact)
- The directory may keep other phase-specific notes but progress is cleaned
</step>


## Examples

<bad>
"All sprints commit cleanly. Phase III.C sealed. Updated ROADMAP."

(No file:line evidence. No red line check. No partial categorization.
No friction status verification. No diff proposed — direct write.)
</bad>

<good>
| Sprint | Status | Evidence | Action |
|---|---|---|---|
| FC.1 | Pass | search_vault returns ToolOutput at vault_tools.py:78 | - |
| FC.2 | Accepted Partial | bb-message-artifacts only fires when artifacts non-empty (declared in plan) | Append ACCEPTED_PARTIALS.md |
| FC.3a | Technical Debt | open_vault_note fallback to system default not tested for non-Obsidian users | DEBT-012 |
| FC.4 | Pass | test_router_persona_invariance passes at test_prompt_composer.py:340 | - |

Red Lines:
| OpenAI structured-output API not added | Held | grep "response_format" → 0 hits in src/ |
| artifacts in messages | Held | grep "artifacts" in agent.py message-build paths → 0 hits |

Hard Sealing:
| 3 vault tools return ToolOutput | Pass | vault_tools.py:78,108,141 |
| artifacts NEVER in messages | Pass | grep verified |
| Router persona-invariant | Pass | test passes |

Frictions:
| friction-260506 E3 → SCHEDULED → RESOLVED | docs/logs/friction-260506.md:42 |
| friction-260506 E4 → SCHEDULED → RESOLVED | docs/logs/friction-260506.md:67 |


✅ Sealed. 4 Pass, 1 Accepted Partial, 1 Technical Debt filed.
</good>

## Success Criteria
- [ ] Every sprint in batch has explicit verdict + evidence
- [ ] Every Partial categorized with reason
- [ ] Every red line verified via Grep/Bash
- [ ] Hard sealing conditions checked individually
- [ ] Driving frictions status verified
- [ ] Sealing decision unambiguous (Sealed / Functionally Sealed / NOT Sealed)
- [ ] Proposed diffs for state files output as unified diff
- [ ] No code modifications attempted
- [ ] No state files directly written (proposals only)
