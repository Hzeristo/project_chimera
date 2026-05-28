# EXT.2d — `<thinking>` tag stripping in probe parsing path

- **Status:** Pending
- **Predecessor:** EXT.2c sealed

## Goal

Strip `<thinking>...</thinking>` blocks from `probe_response` before tool call
parsing. `TextSanitizer.strip_reasoning_tags()` already handles this — it just
needs to be called on `probe_response` before `_parse_tool_calls()`.

## Analysis

`strip_reasoning_tags()` (`text_sanitizer.py:143`) already strips `<thinking>`
and `redacted_thinking` tags with up to 64 nested passes. It is called in
`sanitize_messages_history` for history sanitization, but NOT on the raw
`probe_response` before tool call parsing.

Current flow in `_run_theater_inner` (`agent.py:1159`):
```
probe_response = await ...generate_raw_text(...)
planned_calls = self._parse_tool_calls(probe_response)   # ← thinking tags still present
```

`_parse_tool_calls` calls `_strip_markdown_code_for_cmd_extraction` (strips code
fences) then `parse_tool_calls_unified`. Neither strips `<thinking>` blocks.
A `<thinking>` block containing text like `I should call search_vault` will not
cause false positives (no `<tool_call>` inside), but a thinking block that
accidentally contains a `<tool_call>` example would be executed. Stripping first
is the correct defense.

## Files touched

| File | Change |
|---|---|
| `crucible_core/src/oligo/core/agent.py` | One line: strip thinking tags from `probe_response` before `_parse_tool_calls`. |

## Implementation

In `_run_theater_inner`, after receiving `probe_response` and before
`_parse_tool_calls`:

```python
probe_response = TextSanitizer.strip_reasoning_tags(probe_response)
```

That's the entire change. `strip_reasoning_tags` is already imported via
`TextSanitizer` which is already imported in `agent.py`.

The stripped `probe_response` is also used for `backfill_draft` downstream
(`line 1300`) — stripping thinking tags there is correct behavior (thinking
should not appear in the backfilled draft either).

## Red lines

- Do NOT modify `strip_reasoning_tags` itself.
- Do NOT add a new sanitizer method.

## Seal check

- `pytest tests/oligo/` → pre-existing 8 failures only, no new failures
- `grep "strip_reasoning_tags" agent.py` → present in probe path
