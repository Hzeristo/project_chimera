# EXT.2b — Router prompt content paste + verification

- **Status:** Content delivered (user edited router_intro.md.j2 directly)
- **Predecessor:** EXT.2a sealed

## Goal

Verify that the user-provided `router_intro.md.j2` content renders correctly
through `_load_j2()` → `compose()` → `_build_router_system_prompt()`. Update
byte-lock baseline.

## Files touched

| File | Change |
|---|---|
| `crucible_core/src/oligo/prompts/router_intro.md.j2` | Already updated by user. No further edits. |
| `crucible_core/tests/oligo/test_prompt_middleware_regression.py` | Update `MW4_COMBINED_PROMPT_BASELINE_BYTES` to new value after cap removal + new template. |

## Verification steps

1. Run `pytest tests/oligo/test_prompt_middleware_regression.py` — expect only
   `test_mw4_baseline_byte_lock_unchanged` to fail (wrong constant).
2. Print actual byte count from the failure message, update the constant.
3. Re-run — all pass.
4. Confirm `router_intro.md.j2` Jinja2 comments (`{# ... #}`) are stripped by
   `_load_j2()` (they are — Jinja2 renders them to empty string before brace-escape).

## Notes on user-provided template

The template uses `{# ... #}` Jinja2 comment blocks. These render to empty string
at registration time, so they do not appear in the composed prompt. The `_load_j2`
brace-escape step runs on the rendered output, not the source — no issue.

The template ends with a `{# Tool list injected here... #}` comment block. The
actual tool list is injected by `router_tool_registry` component via `{tool_list}`
in `_register_default_components`, not by the template itself. This is correct.

## Seal check

- `pytest tests/oligo/test_prompt_middleware_regression.py` → all pass
- `grep "爬取" src/oligo/prompts/router_intro.md.j2` → present (alias coverage)
