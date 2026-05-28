# EXT.2a — Remove 4000-char cap + budget-shrink loop

- **Commit:** `d458009`
- **Status:** Sealed
- **Files changed:**
  - `crucible_core/src/oligo/core/agent.py` — removed `_ROUTER_SYSTEM_PROMPT_MAX_CHARS = 4000` constant and budget-shrink loop; `_build_router_system_prompt()` now single `compose()` call at `None` budget
  - `crucible_core/tests/oligo/test_prompt_middleware_regression.py` — removed `test_ir1_router_system_message_within_char_cap` (cap gone)

## What was done

Removed the `_ROUTER_SYSTEM_PROMPT_MAX_CHARS = 4000` constant (IR.1 era, DEBT-005) and the 6-budget shrink loop in `_build_router_system_prompt()`. The loop was introduced to prevent context overflow when the router prompt was ~600 tokens; with the new 5500+ token router prompt, the loop is obsolete and the cap would truncate the prompt.

`_build_router_system_prompt()` is now a single `compose()` call with `tool_list_max_chars=None`.

## HSC verification

- `grep "_ROUTER_SYSTEM_PROMPT_MAX_CHARS\|budgets.*3200" src/oligo/core/agent.py` → 0 hits ✓
- `pytest tests/oligo/test_prompt_middleware_regression.py` → 9/9 PASS ✓
