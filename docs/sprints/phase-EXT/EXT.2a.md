# EXT.2a — Structural scaffolding: remove cap + budget-shrink loop

- **Status:** Pending
- **Predecessor:** EXT.1 sealed

## Goal

Remove `_ROUTER_SYSTEM_PROMPT_MAX_CHARS = 4000` and the budget-shrink loop from
`_build_router_system_prompt()`. Replace with a single `composer.compose()` call
at `None` budget. Update the `test_ir1_router_system_message_within_char_cap` test
which asserts `<= 4000`.

## Files touched

| File | Change |
|---|---|
| `crucible_core/src/oligo/core/agent.py` | Remove `_ROUTER_SYSTEM_PROMPT_MAX_CHARS` constant (`line 74`). Rewrite `_build_router_system_prompt()` (`lines 488–519`) to single compose call. |
| `crucible_core/tests/oligo/test_prompt_middleware_regression.py` | Remove or update `test_ir1_router_system_message_within_char_cap` (asserts `<= 4000`; cap is gone). |

## Implementation

`_build_router_system_prompt()` becomes:

```python
def _build_router_system_prompt(self) -> str:
    composer = get_prompt_composer()
    active_ids = self._compute_active_router_components()
    context = self._prompt_context()
    stable, dynamic = composer.compose(
        stage=PromptStage.ROUTER,
        context=context,
        active_ids=active_ids,
    )
    body = f"{stable}\n\n{dynamic}".strip()
    logger.debug("[Prompt] router compose stable_len=%s dynamic_len=%s", len(stable), len(dynamic))
    return body
```

The `__init__` call site (`router_body = self._build_router_system_prompt()`) is unchanged.

## Red lines

- Do NOT touch `compose()` core logic.
- Do NOT modify the theater loop.

## Seal check

- `grep "_ROUTER_SYSTEM_PROMPT_MAX_CHARS\|budgets.*3200" agent.py` → 0 hits
- `pytest tests/oligo/test_prompt_middleware_regression.py` → all pass
