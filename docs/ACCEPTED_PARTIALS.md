# Accepted Partials

Trade-offs explicitly accepted at sprint seal. Do NOT reclassify as debt.

## Phase III.B.3 (2026-05-XX)
- Tool list compression under length budget — reason: length budget harder than args visibility
- Zero-arg emission not asserted vs live LLM — reason: requires live-model CI
- DENIED-only batches no telemetry — reason: denial is not execution
- 0.35s opacity vs 1s removal — reason: animation budget

## Phase III.B.1 (2026-04-XX)
- Wash system outside PromptComposer
- tool_list breaks stable byte-identity under tool-set change
