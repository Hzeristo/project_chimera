## Phase {X.Y} / Sprint {N} Review Verdict

**Sprint:** {sprint_title}
**Modified files:** {n} files, +{added}/-{removed} lines
**Friction reference:** `{friction_id}`

### Dimension Verdicts

| Dim | Name | Status | Evidence | Action |
|---|---|---|---|---|
| 1.1 | {dim_name} | Pass | `{file}:{line}` | - |
| 1.2 | {dim_name} | Accepted Partial | `{file}:{line}` | Reason: {one_line} |
| 2.1 | {dim_name} | Technical Debt | `{file}:{line}` | Filed as DEBT-{id} |
| 3.1 | {dim_name} | Fail | `{file}:{line}` | Patch: {description} |

### Red Lines

| Red Line | Status | Verification |
|---|---|---|
| 不引入 X | Held | `grep "X"` returns 0 hits in src/ |
| 不破坏 Y | Held | `pytest tests/test_y.py` passes |

### Sealing Decision

{ONE_OF}:
- ✅ **Sealed.** All Pass or only Accepted Partials.
- ⚠️ **Functionally Sealed.** {n} Technical Debt items filed: DEBT-{id_1}, ...
- ❌ **NOT Sealed.** Fails detected. Fix required before merge.

### Updates Required

- `docs/ROADMAP.md`: {what_to_change}
- `docs/ACCEPTED_PARTIALS.md`: append {n} entries
- `docs/TECHNICAL_DEBT.md`: append {n} entries

### Proposed Diffs

```diff
{actual_diff_for_state_files}
```