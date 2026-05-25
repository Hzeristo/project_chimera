# FC.4 — Persona/Router separation — test-only

- **Commit:** `9a4e542`
- **Status:** Pass
- **Files changed:** `crucible_core/tests/oligo/test_prompt_composer.py`

## What was done

Added two unit tests to `test_prompt_composer.py` asserting Router persona-invariance (Hard Sealing Condition #3).

**`test_router_persona_invariance`**
- Builds two `PromptComposer` instances with identical ROUTER components; one receives `persona` in context, the other does not.
- Asserts `stable` and `dynamic` sections are byte-identical across both calls.
- Docstring notes direct-mode (Rust `lib.rs:706-735`) persona injection is out-of-scope per audit Q6 caveat.

**`test_router_drops_persona_component_with_warning`**
- Registers a `{persona}`-bearing component for `PromptStage.ROUTER`.
- Asserts the persona value does not appear in the composed output.
- Asserts a `WARNING`-level log entry names the dropped component id (`leaky_persona`).
- Uses `caplog` fixture; no new test infrastructure.

## Verification

- `pytest tests/oligo/test_prompt_composer.py -k persona` — 2/2 PASS
- No source changes; red line "do NOT modify prompt_composer.py or agent.py" honored.

## Accepted partials

None.

## Planning deviation

None.
