# Phase Review Verdict: Phase {X.Y} — {phase_name}

**Audit reference:** `docs/audits/{prerequisite-sprint-id}.md` (e.g., `docs/audits/IV.A.0.md`)
**Batch plan reference:** `docs/plans/Phase-{X.Y}-batch.md` (e.g., `docs/plans/Phase-IV.A-batch.md`)
**Sprints in batch:** {sprint_id_1}, {sprint_id_2}, ..., {sprint_id_N}
**Sprints completed:** {n} of {total}
**Batch history source:** sprint summaries (`docs/sprints/phase-{X.Y}/`) — or "reconstructed from phase doc" if the sprint dir was missing/empty
**Date:** YYYY-MM-DD

---

## Per-Sprint Verdicts

| Sprint | Status | Evidence | Action |
|---|---|---|---|
| {sprint_id} | Pass | `{file}:{line}` | - |
| {sprint_id} | Accepted Partial | `{file}:{line}` | Reason: {one_line}; appended to ACCEPTED_PARTIALS.md |
| {sprint_id} | Technical Debt | `{file}:{line}` | Filed as DEBT-{id} |
| {sprint_id} | Fail | `{file}:{line}` | Patch proposed: {description} |
| {sprint_id} | Not Run | (batch halted at {prior_sprint}) | Sprint deferred |

---

## Phase-Wide Red Lines

| Red Line | Status | Verification |
|---|---|---|
| {red_line_1} | Held | `PowerShell("Select-String ...")` returned 0 hits |
| {red_line_2} | Held | `pytest tests/test_x.py` passes |
| {red_line_3} | **Violated** | `{file}:{line}` shows {what} |

---

## Hard Sealing Conditions

| Condition | Status | Verification |
|---|---|---|
| {condition_1} | Pass | `{file}:{line}` |
| {condition_2} | Pass | Test {test_name} passes |
| {condition_3} | Pass | Grep verified |

---

## Driving Friction Resolution

| Friction | Original Status | Current Status | Evidence |
|---|---|---|---|
| {friction_id_1} | SCHEDULED | RESOLVED | sprint {N} commit {hash}, addresses sub-root {x} |
| {friction_id_2} | SCHEDULED | RESOLVED | sprint {M} commit {hash} |
| {friction_id_3} | SCHEDULED | DEFERRED | reason: {one_line} |

---

## Sealing Decision

{ONE_OF}:

✅ **Sealed.** All Pass or only Accepted Partials. Hard sealing conditions met.
   Frictions resolved or deferred with reason.

⚠️ **Functionally Sealed.** Pass + Accepted Partials + {n} Technical Debt items filed.
   Phase moves forward; debt tracked.
   Filed: DEBT-{id_1}, DEBT-{id_2}.

❌ **NOT Sealed.** Reason:
   - {fail_or_violation}
   - Required action: {minimal_patch_description}

---

## State File Updates

### Auto-applied (already written + staged by phase_review; no approval needed)

Per the skill's state_write_authority these were appended/flipped and staged —
listed for the record, NOT for manual application:

- `docs/ACCEPTED_PARTIALS.md` — appended:
  - {sprint_id}: {description} — reason: {why}
- `docs/TECHNICAL_DEBT.md` — appended:
  - DEBT-{id} | Phase {X.Y} review | {description} | {Low/Med/High} | {resolution_plan}
- Friction log — flipped OPEN/SCHEDULED → RESOLVED:
  - {friction_id}: `{file}:{line}`

### Proposed for approval (decision-bearing; apply only after user approves)

#### `docs/ROADMAP.md`

```diff
{diff_for_phase_status_change}
```

#### Friction reclassifications (not direct phase-resolutions)

```diff
{diff_for_friction_reclassification, e.g. OPEN → WONTFIX}
```

#### Any change that conflicts with existing content

```diff
{diff_for_conflicting_change}
```

---

## Audit-to-Implementation Trace

Sanity check: every audit cross-finding either led to a sprint or was explicitly deferred.

| Audit Finding | Outcome |
|---|---|
| Cross-finding 1: {summary} | Addressed by {sprint_id} |
| Cross-finding 2: {summary} | Deferred to Phase {Y}, reason: {why} |
| Cross-finding 3: {summary} | Addressed by {sprint_id} |

---

## Appendix: Worked Example (reference — do not fill)

A bad verdict and the good verdict it should have been. Illustration only.

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
| OpenAI structured-output API not added | Held | Select-String "response_format" → 0 hits in src/ |
| artifacts in messages | Held | Select-String "artifacts" in agent.py message-build paths → 0 hits |

Hard Sealing:
| 3 vault tools return ToolOutput | Pass | vault_tools.py:78,108,141 |
| artifacts NEVER in messages | Pass | Select-String verified |
| Router persona-invariant | Pass | test passes |

Frictions:
| friction-260506 E3 → SCHEDULED → RESOLVED | docs/logs/friction-260506.md:42 |
| friction-260506 E4 → SCHEDULED → RESOLVED | docs/logs/friction-260506.md:67 |


✅ Sealed. 4 Pass, 1 Accepted Partial, 1 Technical Debt filed.
</good>

---

*Generated by chimera-sprint-discipline phase_review mode.*
