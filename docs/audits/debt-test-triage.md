# Test Triage — Pre-existing Failures
*Date: 2026-06-20 | Read-only — no fixes applied.*

## Discrepancy note
Session pre-compaction cited 8 pre-existing failures. Full run finds **12**.
The 4 extra are in `test_tool_execution.py` (the `run_theater_*` trio + `unknown_tool`).
All 12 are confirmed to predate Tier 1 commits (git-stash baseline).

---

## Per-test table

| # | Test | Failing assertion | Q1 — What it asserts | Q2 — Is assertion still valid? | Class | Proposed action |
|---|------|-------------------|----------------------|-------------------------------|-------|-----------------|
| 1 | `test_obsidian_graph_query_adapter_not_set` | `assert "Vault adapter not initialized" in out` → `out` is `ToolOutput(...)` | `in` on a ToolOutput object | No — `obsidian_graph_query` now returns `ToolOutput`, not `str`; `in` over a Pydantic model checks field-name iteration, not `.text` | **B** | Update test: `assert "Vault adapter not initialized" in out.text` |
| 2 | `test_obsidian_graph_query_no_nodes` | `assert "[Graph Query] No nodes found" in out` | Same — `in` on a ToolOutput | No — same return-type change | **B** | Update: `assert "..." in out.text` (×2 assertions) |
| 3 | `test_obsidian_graph_query_formats_nodes_and_links` | `assert "[Graph Query] Found 1 nodes" in out` | Same | No — same return-type change | **B** | Update all `in out` → `in out.text` |
| 4 | `test_obsidian_graph_query_truncates_beyond_ten` | `assert "Found 12 nodes" in out` | Same | No — same return-type change | **B** | Update all `in out` → `in out.text` |
| 5 | `test_mw4_baseline_byte_lock_unchanged` | `assert 7459 == 7492` | Combined Router+Final prompt UTF-8 size equals the MW.4 locked baseline | No — template changed (−33 bytes); the test's own docstring says "update `MW4_COMBINED_PROMPT_BASELINE_BYTES` and docs when templates change intentionally" | **B** | Update `MW4_COMBINED_PROMPT_BASELINE_BYTES = 7492` → `7459` in `test_prompt_middleware_regression.py:44`; update `docs/PROMPT_MIDDLEWARE.md` lock comment |
| 6 | `test_execute_tool_calls_unknown_tool_returns_error` | `assert results[0].status == ToolCallStatus.ERROR` | Unknown tool name returns `ERROR` status | No — `_execute_tool` now returns `("Error: Tool '...' not recognized...", None)` without raising; `_run_one` always returns `SUCCESS` when no exception is raised; error is in `raw_result` | **B** | Update test: assert `results[0].status == ToolCallStatus.SUCCESS` and `"not recognized" in results[0].raw_result` |
| 7 | `test_run_theater_with_tool_calls_executes_and_streams` | `assert client.final_call_count == 1` → got 2 | After one tool turn, final LLM is called exactly once | Yes — the contract should be probe×1 → tool → final×1. **Root cause: `_step_execute` regression** — `patched` only appends long-running tool results; normal tools dropped silently → `execute_result.executed_results` is empty → tool results never rendered into messages → theater loops again, mock router invocation without "Chimera OS local router" in turn-2 system prompt counted as final | **A** | Fix `agent.py:_step_execute` — append all non-long-running results to `patched` unconditionally |
| 8 | `test_run_theater_no_tool_passes_through_to_final_stream` | `assert "Hello from the other side" in "".join(chunks)` | Final text appears verbatim in joined stream | No — stream now emits SSE frames (`event: bb-stream-chunk\ndata: {"content": "Hel"}\n\n …`); text is fragmented into 3-char JSON-encoded chunks; verbatim substring never contiguous in joined output | **B** | Update test: parse SSE frames and reconstruct text, e.g. extract all `bb-stream-chunk` data values and join content fields |
| 9 | `test_run_theater_natural_language_probe_backfilled_for_final` | `assert "Polished persona answer" in "".join(chunks)` | Same — final text in joined stream | No — same SSE framing issue | **B** | Same fix as #8 |
| 10 | `test_artifacts_event_emitted_once_with_dedup` | `assert len(artifact_events) == 1` → got 0 | One `bb-message-artifacts` SSE frame emitted per theater run with vault-note artifacts | Yes — FC.2a contract. **Root cause: same `_step_execute` regression**. `execute_result.executed_results` is empty for non-long-running tools → `_step_wash` receives `[]` → `_accumulate_artifacts([])` → `_session_artifacts` stays empty → no frame emitted | **A** | Fix `agent.py:_step_execute` (same as #7) |
| 11 | `test_artifacts_event_after_final_chunks_no_python_done` | `assert artifact_idx is not None` | One `bb-message-artifacts` frame exists after the last `bb-stream-chunk` | Yes — FC.2a contract. Same root cause as #10 | **A** | Fix `agent.py:_step_execute` (same as #7) |
| 12 | `test_session_artifacts_never_in_messages` | `assert agent._session_artifacts` → got `[]` | `_session_artifacts` is non-empty after a tool turn with `obsidian_graph_query` | Yes — FC.2a contract. Same root cause as #10 | **A** | Fix `agent.py:_step_execute` (same as #7) |

---

## Summary

**A=4, B=8**

### Root-cause grouping

#### Group 1 — A: `_step_execute` regression (fixes tests #7, #10, #11, #12)

`agent.py` lines 1172–1182:

```python
patched: list[ExecutedToolResult] = []
has_long_running = False
for er in executed_results:
    raw = er.raw_result or ""
    if registry.is_long_running(er.tool_name):   # ← only long-running tools appended
        m = re.search(r'[0-9a-f]{8}', raw)
        if m:
            er = er.model_copy(update={"task_id": m.group(0)})
            has_long_running = True
            patched.append(er)
return ExecuteResult(executed_results=patched, ...)
```

Normal (non-long-running) tools are silently dropped from `patched`.
`execute_result.executed_results` is always `[]` for every standard tool call.
Downstream: `_step_wash([], ...)` → `_accumulate_artifacts([])` → `_session_artifacts = []`
→ no `bb-message-artifacts` event; tool results not rendered into messages; theater state corrupt.

Fix: append non-long-running results without `task_id` patch:
```python
for er in executed_results:
    raw = er.raw_result or ""
    if registry.is_long_running(er.tool_name):
        m = re.search(r'[0-9a-f]{8}', raw)
        if m:
            er = er.model_copy(update={"task_id": m.group(0)})
            has_long_running = True
    patched.append(er)
```

---

#### Group 2 — B: `obsidian_graph_query` return-type change (fixes tests #1–4)

`vault_tools.obsidian_graph_query` now returns `ToolOutput`, not `str`.
All four tests do `assert "..." in out` where `out: ToolOutput`.
Pydantic v2 `__contains__` iterates field names → assertion always False.
Fix: `in out` → `in out.text` in all four tests.

---

#### Group 3 — B: SSE stream format change (fixes tests #8–9)

`run_theater()` now yields SSE-framed chunks (`event: bb-stream-chunk\ndata: {"content": "Hel"}\n\n`).
Tests use `"text" in "".join(chunks)` expecting bare text.
Fix: parse SSE frames in tests to extract content fields before asserting.

---

#### Group 4 — B: `ToolCallStatus` contract change for unknown tools (fixes test #6)

`_execute_tool` returns `("Error: Tool '...' not recognized", None)` for unknown tools instead of raising.
`_run_one` therefore returns `ToolCallStatus.SUCCESS` (error is in `raw_result`).
The render layer uses `_success_payload_behaves_like_tool_failure()` to detect this.
Fix: update test to assert `SUCCESS` status + error substring in `raw_result`.

---

#### Group 5 — B: MW.4 prompt byte-lock drift (fixes test #5)

Prompt templates changed by −33 bytes. Test explicitly documents it must be updated whenever templates change intentionally. Fix: update `MW4_COMBINED_PROMPT_BASELINE_BYTES` from `7492` to `7459`.
