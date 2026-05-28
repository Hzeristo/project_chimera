# EXT.2d — Strip `<thinking>` tags from probe_response before parsing

- **Commit:** `b757ced`
- **Status:** Sealed
- **Files changed:**
  - `crucible_core/src/oligo/core/agent.py` — one line added in `_run_theater_stream` before `_parse_tool_calls`

## What was done

Added `probe_response = TextSanitizer.strip_reasoning_tags(probe_response)` immediately after the probe log line and before `_parse_tool_calls`. `strip_reasoning_tags` already handles `<thinking>` and `redacted_thinking` tags with up to 64 nested passes — no new logic needed.

The stripped `probe_response` flows through to `backfill_draft` downstream, which is correct: thinking blocks should not appear in the backfilled draft either.

## HSC verification

- `grep "strip_reasoning_tags" src/oligo/core/agent.py` → present in probe path ✓
- `pytest tests/oligo/` → 8 pre-existing failures only, no new failures ✓
