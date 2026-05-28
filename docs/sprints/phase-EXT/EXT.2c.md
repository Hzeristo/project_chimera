# EXT.2c — First-turn vs continuation system prompt separation

- **Status:** Pending
- **Predecessor:** EXT.2b sealed

## Goal

Add `router_continuation.md.j2` (~200 tokens). Modify the theater loop so that:
- Turn 1 (`turn == 1` after increment): uses the existing full router system prompt
  already in `self.messages[0]`.
- Turn 2+ (`turn > 1`): replaces `self.messages[0]` with the continuation prompt
  before the probe call.

## Files touched

| File | Change |
|---|---|
| `crucible_core/src/oligo/prompts/router_continuation.md.j2` | New file. ~200-token focused evaluation directive. |
| `crucible_core/src/oligo/core/agent.py` | Add `_build_router_continuation_prompt()`. Modify theater loop to swap `self.messages[0]` on `turn > 1`. |

## Template content: `router_continuation.md.j2`

```
You are the Chimera OS router, mid-turn.

Tool results are now in the conversation. Evaluate them:

1. Do the results answer the user's question sufficiently?
   - Yes → output <PASS> or a brief synthesis draft for Final.
   - No → call another tool to fill the gap.
   - Partial → call a follow-up tool for the missing piece.

2. If a long-running tool returned a task_id, call check_task_status
   with that task_id to report progress.

3. Do NOT repeat a tool call with identical args if it already returned
   a result in this conversation.

Output: tool call(s) OR <PASS> OR a brief synthesis draft. Nothing else.
```

## Implementation in agent.py

Add method:
```python
def _build_router_continuation_prompt(self) -> str:
    return _load_j2("router_continuation.md.j2")
```

In `_run_theater_inner`, at the top of the `while` loop, after `turn += 1`:
```python
if turn > 1:
    self.messages[0] = ChatMessage(
        role="system",
        content=self._build_router_continuation_prompt(),
    )
```

This is the minimal touch: one conditional assignment before the probe call.
`self.messages[0]` is always the router system message (set in `__init__`).

## Red lines

- Do NOT change `__init__` message construction.
- Do NOT touch `compose()`.
- The swap must happen BEFORE `_apply_history_sanitizer_to_messages()` in the loop.

## Seal check

- `pytest tests/oligo/test_prompt_middleware_regression.py tests/oligo/test_prompt_composer.py` → all pass
- Manual: two-turn conversation (tool call turn 1, follow-up turn 2) logs show
  `[Router] ROUTER SYS` first 150 chars differ between turn 1 and turn 2.
