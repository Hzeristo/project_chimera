# EXT.2b — Verify user-provided router_intro.md.j2 + update byte-lock

- **Commit:** `d458009`
- **Status:** Sealed
- **Files changed:**
  - `crucible_core/tests/oligo/test_prompt_middleware_regression.py` — byte-lock baseline 2594 → 7492

## What was done

User delivered `router_intro.md.j2` content via direct file edit (2026-05-28). The template uses Jinja2 comment blocks (`{# ... #}`) which render to empty string at registration time. `_load_j2()` brace-escapes the rendered output correctly.

Byte-lock updated to 7492 (delta +4898 bytes from new router_intro content).

The `test_overall_prompt_length_within_mw4_migration_budget` 110% guard was also failing — resolved by updating the baseline constant.

## HSC verification

- `pytest tests/oligo/test_prompt_middleware_regression.py` → 9/9 PASS ✓
- `grep "爬取" src/oligo/prompts/router_intro.md.j2` → present ✓
