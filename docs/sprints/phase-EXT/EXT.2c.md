# EXT.2c — router_continuation.md.j2 + theater loop turn-based swap

- **Commit:** `b757ced`
- **Status:** Sealed
- **Files changed:**
  - `crucible_core/src/oligo/prompts/router_continuation.md.j2` — new file, ~200 tokens
  - `crucible_core/src/oligo/core/agent.py` — `_build_router_continuation_prompt()`; turn-based swap in `_run_theater_stream`

## What was done

Added `router_continuation.md.j2`: a focused evaluation directive for turn 2+. Covers the 4-case decision framework (sufficient / partial / empty / task_id), constraints carried from first turn, and output format.

In `_run_theater_stream`, after `turn += 1`, added:
```python
if turn > 1:
    self.messages[0] = ChatMessage(
        role="system",
        content=self._build_router_continuation_prompt(),
    )
```

`_build_router_continuation_prompt()` uses `_load_md()` (not `_load_j2()`) because the continuation template has no Jinja2 syntax — plain text, no brace-escape needed.

Saves ~1400 tokens per subsequent turn by replacing the full 1650-token router_intro with the ~200-token continuation prompt.

## HSC verification

- `pytest tests/oligo/test_prompt_middleware_regression.py tests/oligo/test_prompt_composer.py` → 20/20 PASS ✓
- Pre-existing failures unchanged: 8
