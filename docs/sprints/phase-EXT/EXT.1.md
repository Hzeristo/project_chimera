# EXT.1 — Externalize inline prompt constants

- **Commit:** `10a282a`
- **Status:** Sealed
- **Files changed:**
  - `crucible_core/src/oligo/core/prompt_composer.py` — replaced 3 module-level constants with `_load_j2`/`_load_md` loaders; added Jinja2 env; removed `retrieval_context_demo` dead component
  - `crucible_core/src/oligo/core/agent.py` — replaced inline wash f-string with `_load_md("wash_system_prompt.md").format(...)`
  - `crucible_core/src/oligo/prompts/` — 9 new template files (3 × `.md.j2`, 6 × `.md`)
  - `crucible_core/tests/oligo/test_prompt_middleware_regression.py` — byte-lock baseline 2492 → 2594
  - `crucible_core/tests/oligo/test_prompt_composer.py` — removed 2 tests asserting on dead `retrieval_context_demo` component; updated singleton test

## What was done

**Template files created** under `src/oligo/prompts/`:

| File | Flavor | Source constant |
|---|---|---|
| `router_intro.md.j2` | Jinja2 (registration-time) | `ROUTER_INTRO` |
| `router_post_tools.md.j2` | Jinja2 (registration-time) | `ROUTER_POST_TOOLS` |
| `final_guardrail.md.j2` | Jinja2 (registration-time) | `FINAL_GUARDRAIL_TEXT` |
| `router_skill_directive.md` | str.format (compose-time) | inline in `_register_default_components` |
| `final_system_core.md` | str.format (compose-time) | inline in `_register_default_components` |
| `final_skill_directive.md` | str.format (compose-time) | inline in `_register_default_components` |
| `final_persona_override.md` | str.format (compose-time) | inline in `_register_default_components` |
| `final_authors_note.md` | str.format (compose-time) | inline in `_register_default_components` |
| `wash_system_prompt.md` | str.format (call-time) | inline f-string in `_wash_tool_result` |

**Loader design:** `_load_j2()` renders Jinja2 at registration time and escapes all braces (`{` → `{{`) so the result survives `str.format(**context)` at compose time without KeyError. `_load_md()` reads the file as-is for compose-time `.format()`. `compose()` core logic untouched.

**Dead code removed:** `retrieval_context_demo` xml_structured component — never included by `_compute_active_router_components` in production (`agent.py:441–454`). Two tests that asserted on it removed.

**Guardrail update:** `final_guardrail.md.j2` adds `<tool_call name="...">` XML prohibition (audit cross-finding 2). Byte-lock delta +102 bytes.

## HSC verification

- `grep -r "ROUTER_INTRO\|ROUTER_POST_TOOLS\|washer_sys\s*=\s*(" src/oligo/` → 0 hits ✓
- `pytest tests/oligo/test_prompt_middleware_regression.py` → 10/10 PASS ✓
- Pre-existing failures unchanged: 8 (test_tool_execution × 4, test_obsidian_graph_query × 4)

## Planning deviations

None. `compose()` untouched per red line. No new Python dependencies (Jinja2 already present). Wash prompt correctly routed to `.md` + str.format, not Jinja2.
