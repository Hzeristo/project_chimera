## Self-Review Verdict

**Scope:** {git_range_or_files}

### Rule Conformance Table

| Rule | Status | Evidence | Recommended Fix |
|---|---|---|---|
| layering_direction | Pass | - | - |
| function_naming | Pass | All new ≤ 25 chars | - |
| abstraction_threshold | Fail | `agent.py:412` extracts helper used once | Inline `_extract_X` |
| exception_handling | Pass | `agent.py:520` re-raises CancelledError | - |
| pydantic_defaults | Accepted Partial | `schemas.py:340` no Field description on `_internal_id` | Reason: internal-only |
| ui_tokens | N/A | No CSS changes |  |
| logging_format | Fail | `agent.py:780` log without bracket prefix | Prepend `[Final]` |
| no_opportunistic_refactor | Pass | All within sprint scope | - |

### Recommendation

{ONE_OF}:
- ✅ **Apply as-is.** All Pass or Accepted.
- 🛠️ **Apply N fixes.** {n} violations:
  - Fix 1: {description} (`{file}:{line}`)
  - Fix 2: {description} (`{file}:{line}`)
- ❌ **Block merge.** Critical violations:
  - {critical_violation_description}

If "Apply N fixes" selected and user confirms, transition to `code_modification` mode.