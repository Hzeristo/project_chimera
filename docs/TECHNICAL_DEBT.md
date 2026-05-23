# Technical Debt

Items found during review or use that are NOT accepted partials. These are deficiencies with eventual resolution intent.

**Distinction:** Accepted Partials are trade-off decisions. Technical Debt are unresolved problems.

---

## Open

| ID | Source | Description | Priority | Resolution Plan |
|---|---|---|---|---|
| DEBT-001 | III.B.1 review (MW.4) | 6 async tests in `tests/oligo/test_tool_execution.py` lack `@pytest.mark.asyncio`; pytest skips them silently | Medium | Next debt week. Add markers, fix any latent failures exposed |
| DEBT-002 | Vibe coding residue | Long-name single-call-site functions across `agent.py` and earlier prompt code (e.g., `_build_router_system_prompt_with_skill_override_and_allowed_tools_filtered_by_whitelist` style) | Low | Touch-and-repair: rename when modifying surrounding code, no dedicated sprint |
| DEBT-003 | Phase II.E friction E2 | `search_vault` keyword-only matching produces low-precision results on conceptual queries | Low | Defer to Phase IV (Exocortex retrieval). Marked OPEN to remind that friction is real but not currently scheduled |
| DEBT-004 | III.B.2 audit | `_execute_tool` no longer needs to re-parse `raw_args` after `PlannedToolCall.args` carries the dict, but legacy parse path remains | Low | Touch-and-repair when next modifying tool dispatch |
| DEBT-005 | III.B.3 audit | Router prompt's args description rendering occasionally falls back to compact mode under non-extreme tool counts; threshold tuning needed | Low | Observe across Phase III.C usage; tune if frictions arise |
| DEBT-006 | UI residue | Stage cards persist across sessions until manually cleared; no automatic cleanup older than N days | Very Low | Backlog |
| DEBT-007 | III.B.3 review (IR.3.1 boundary case) | Tool telemetry doesn't cover *parsing-stage* failures (malformed XML before any plan is built); only execution-stage covered | Low | Next debt week if frictions arise |
| DEBT-008 | PaperMiner-era env / CLAUDE.md TBD | Replace inherited PaperMiner conda environment with a clean `uv`-managed `.venv` pinned to `pyproject.toml` only | Medium | Debt week: audit conda packages and classify Chimera-used vs leftover; create `.venv/` via uv; run full test + smoke suite; set declared env path in CLAUDE.md; archive (not delete) old conda env |
| DEBT-009 | FC.2a smoke work (May 2026) | FC.2a smoke test stalled: mock never transitions to PASS, exhausting `max_turns`. Roots: (a) `conftest.MockLLMClient` lacks PASS-switch logic (b) workaround only exists in `agent.py` `__main__`; testing via `__main__` is an antipattern (c) `test_run_theater_with_tool_calls_executes_and_streams` may hide the mock gap | Medium | Unify mock harness; add PASS-switch (or equivalent) on `MockLLMClient`; debt week remove `__main__`-oriented test code from agent |

---

## Resolved

| ID | Resolved in | Commit | Original Description |
|---|---|---|---|
| DEBT-pre-001 | Phase I M1 | `{commit}` | LLM main path had no outer timeout, allowing indefinite hangs |
| DEBT-pre-002 | Phase I M1 | `{commit}` | `_run_one` used `except BaseException` swallowing `KeyboardInterrupt` |
| DEBT-pre-003 | Phase I M1 | `{commit}` | `web_search` invoked `ddgs.text()` synchronously, blocking event loop |
| DEBT-pre-004 | Phase I M2 | `{commit}` | Hardcoded `127.0.0.1:33333` in Rust + Python |
| DEBT-pre-005 | Phase I M2 | `{commit}` | `_TOOL_TIMEOUT_MESSAGE` hardcoded "45 seconds" decoupled from configurable deadline |
| DEBT-pre-006 | Phase III.A Step 0 | `{commit}` | `<CMD:...>` literal mentions in natural-language explanations triggered actual execution |
| DEBT-pre-007 | Phase III.A Step 0 | `{commit}` | LLM hallucinated tool names not in `TOOL_REGISTRY` |
| DEBT-pre-008 | Phase III.B.1 | `{commit}` | Persona / skill_override / system_core hand-concatenated across 5+ sites |
| DEBT-pre-009 | Phase III.B.2 | `{commit}` | Argument JSON parsing failed on smart quotes, code fences, trailing commas |

---

## Triage Rules

- **Critical:** Block next sprint. Fix immediately.
- **High:** Fix within current phase before seal.
- **Medium:** Fix in next dedicated debt week.
- **Low:** Fix opportunistically when modifying nearby code.
- **Very Low:** Backlog. Re-evaluate annually.

---

## Debt Week Process

When backlog accumulates:
1. Schedule a Use Week (no new code) followed by a Debt Week (only this list).
2. Pick highest-priority Open items.
3. For each: read, scope, sprint, fix, verify.
4. Move to Resolved with commit hash.
5. New friction in Use Week may surface DEBT-NEW items; append, don't preempt the queue.

---

*Update protocol: New entries via `chimera-sprint-discipline` review process. Resolved entries moved by author at fix-commit time.*
