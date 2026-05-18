# Audit Report: Phase III.C — FC.0

**Scope:** Read-only audit of tool-return shape, SSE/Tauri/Svelte artifact pipeline surface, persona injection timing, and message-delete persistence path. Prerequisite for FC.1–FC.5.

**Date:** 2026-05-18
**Mode:** Read-only — no fix proposals, no code modifications.

---

## Files read

| Path | Lines |
|---|---|
| `crucible_core/src/oligo/tools/vault_tools.py` | 169 |
| `crucible_core/src/oligo/tools/web_search.py` | 80 |
| `crucible_core/src/oligo/tools/miner_tools.py` | 157 |
| `crucible_core/src/oligo/tools/registry.py` | 294 |
| `crucible_core/src/oligo/core/agent.py` | 1456 |
| `crucible_core/src/oligo/core/prompt_composer.py` | 437 |
| `crucible_core/src/oligo/api/server.py` | 224 |
| `crucible_core/src/oligo/server.py` | 6 (re-export) |
| `astrocyte/src-tauri/src/llm_client.rs` | 377 |
| `astrocyte/src-tauri/src/state.rs` | 102 |
| `astrocyte/src-tauri/src/lib.rs` | 1534 |
| `astrocyte/src-tauri/src/memory.rs` | grep-only (lines 60–94, 267, 374, 420) |
| `astrocyte/src/routes/+page.svelte` | grep + targeted reads (lines 14–36, 1370–1530) |
| `astrocyte/src/app.css` | grep on token names (lines 97–125, etc.) |

---

## Findings

| Q# | Question | Answer | Evidence | Risk |
|---|---|---|---|---|
| Q1 | What concrete shape does each registered tool return? | All 8 registered tools have signature `async def ... -> str`. The registry's `ToolFn` type alias is `Callable[..., Awaitable[str]]`. | `registry.py:22`; `vault_tools.py:41,68,91,114`; `web_search.py:44`; `miner_tools.py:13,70,122` | Low |
| Q1.sub | Any tool already returns richer structure (dict/TypedDict/Pydantic)? | **No.** Zero tools return non-`str`. The closest "structured" surface is the **adapter** layer: `_VaultToolPort.query_graph` returns `list[dict[str, Any]]`, but `obsidian_graph_query` flattens it to a formatted string before returning. So FC.1's `ToolOutput` is a **genuinely new opt-in shape**, not alignment with an existing convention. | `vault_tools.py:15-20` (adapter signature); `vault_tools.py:140-168` (str flattening); `registry.py:22` (`ToolFn` alias) | Low |
| Q2 | How does `_execute_tool` consume tool returns and where is the LLM-facing string built? | `_execute_tool` calls `fn(**args)` then `out = str(result)` (the str-ification choke point). Result is wrapped into `ExecutedToolResult(raw_result=str(raw), ...)`. LLM-facing string is assembled by `_render_tool_results_for_llm`, which serializes each row via `_format_one_tool_result_xml`. The body is taken from `payload = er.washed_result or er.raw_result or er.error_message; result_body = str(payload)`. | `agent.py:646-672` (`_execute_tool`); `agent.py:783-792` (ExecutedToolResult construction); `agent.py:1040-1085` (`_render_tool_results_for_llm`); `agent.py:255-279` (`_format_one_tool_result_xml`); `agent.py:1053-1057` (str coercion) | Low |
| Q3 | What SSE events does the backend emit per turn, and what is their order around `bb-stream-done`? | Per turn, in order: `__SYS_TOOL_CALL__` data frames (router/tool/wash telemetry), `bb-tool-start` per planned call, `bb-tool-done` per executed call, then either (a) loop back if more tool calls, or (b) `__SYS_TOOL_CALL__` final-stage frame, then **chunked** `bb-stream-chunk` frames. **`bb-stream-done` is NOT emitted by the inner theater loop on success** — the loop simply `return`s and the stream closes; the success-path `bb-stream-done "DONE"` is emitted by Rust at the Tauri layer (`lib.rs`), not by `server.py`. `server.py` only emits `bb-stream-done` on error/disconnect. **Implication for FC.2:** there is no in-Python "before bb-stream-done" hook on the success path; the artifact emission must happen at the **last yield of `_run_theater_stream`** before its `return` at `agent.py:1356`, or be intercepted by the Rust `stream_oligo_agent` before it returns `Ok(Some(...))`. | `agent.py:1133-1264` (per-turn telemetry); `agent.py:735-743,799-810,841-852` (`bb-tool-start`/`done`); `agent.py:1336-1353` (final stage frames + chunks); `agent.py:1355-1356` (return without DONE); `server.py:140-153` (error-only DONE in safe wrapper); `llm_client.rs:294-306` (Rust handles server-emitted DONE); `lib.rs:979` (Rust emits success-path `"DONE"`) | Med |
| Q4 | How does `llm_client.rs` forward backend SSE, and what is the `Message` struct? | `stream_oligo_agent` matches on `msg.event`: `bb-stream-done` → JSON-decode + `app.emit`; `bb-tool-start`/`bb-tool-done` → forward as same-named Tauri events; otherwise data is parsed as JSON `{content: ...}` — `__SYS_TOOL_CALL__` prefix → `bb-sys-event`, else → `bb-stream-chunk`. **Unknown events fall through unhandled** (no else branch), so a new `bb-message-artifacts` event will not be forwarded by default — it requires an explicit handler. `state::Message` is `{ role: String, content: String }` with no id, timestamp, or artifacts field. `Session { history: Vec<Message> }`. | `llm_client.rs:286-358` (event dispatch); `llm_client.rs:294-324` (named event branches); `state.rs:14-18` (`Message`); `state.rs:20-23` (`Session`) | Med |
| Q5 | Frontend `Message` type and available design tokens? | `+page.svelte` uses `HistoryEntry` (not `Message`): `{id, sender, text, timestamp, persona?, isLoading?, streamAborted?, skill_id?, tokens?, feedback?, stage?, tool_name?, decision?, stage_feedback?}`. `Sender` is the discriminator: `'system' \| 'user' \| 'bb' \| 'system_log' \| 'error' \| 'stage_card'`. **No `artifacts` field exists**. Available design tokens in `app.css`: borders (`--border-hud`, `--border-hud-emphasis`, `--border-muted`, `--border-active`, `--border-neutral`, `--border-scrollbar`); spacing (`--space-1`..`--space-6` = 4..24px); radii (`--radius-xs`=2, `--radius-3`=3, `--radius-sm`=4, `--radius-md`=6, `--radius-lg`=8); color reference `var(--astrocyte-neural-purple)` is in scope at `app.css:100` (palette defined elsewhere — full palette read deferred to FC.3 plan time per audit scope). | `+page.svelte:14-36`; `app.css:97-125` (token block) | Low |
| Q6 | At which `PromptStage` does `final_persona_override` inject? Does Router observe persona today? | `final_persona_override` is registered with `stage=PromptStage.FINAL` and `priority=60`. Router actively filters out persona-bearing components: any component whose `template` is `str` and contains `"{persona}"` is dropped at compose time when `stage == ROUTER`, with a warning log. `_compute_active_router_components` never adds persona-related ids. **Conclusion: persona is already FINAL-only on the Oligo path.** Caveat: the **direct-mode** (non-Oligo) path in Rust assembles `[PERSONA OVERRIDE]` into a single system message via `compose_prompt_injection_system` — that path bypasses the Python composer entirely and is outside the agent's Router stage. | `prompt_composer.py:402-409` (registration); `prompt_composer.py:271-282` (Router filter); `agent.py:433-446` (`_compute_active_router_components`); `agent.py:448-461` (`_compute_active_final_components`); `lib.rs:706-735` (direct-mode persona injection — out of FC.4 scope) | Low |
| Q7 | How is session/message persistence implemented and what identifies a deletable message vs a tool/stage row? | Persistence: per-session JSONL files written by `append_session_entries` (`memory.rs:420`), entries typed as `ChatEntry { id, timestamp, role, content, session_id, persona? }` (`memory.rs:66-76`). Deletion flow already wired: Tauri command `delete_chat_message(session_id, msg_id)` exists, calls `delete_entry`, then reloads `state.set_history_for_session`. Frontend `deleteMessage` already calls it. **Stage cards / tool-strip / system_log rows are webview-only** — `dispatchEvaluate` calls `stripStageCards(history)` before each turn, and the persistence comment at `lib.rs:691` declares "JSONL / timeline persistence is strictly **user + bb (assistant)** turns only." Identity: `HistoryEntry.id` is a UUID per `makeId()`; `ChatEntry.id` matches it. **FC.5 is materially already implemented** — see Notable cross-finding 1 below. | `memory.rs:66-76` (`ChatEntry`); `memory.rs:267` (`delete_entry`); `memory.rs:374,420` (read/append); `lib.rs:1060-1109` (`delete_chat_message` command); `lib.rs:691` (persistence boundary comment); `+page.svelte:1373-1387` (`deleteMessage`); `+page.svelte:1498-1508` (`onAiAction`); `+page.svelte:1427` (`stripStageCards` invocation) | Low |
| Q8 | How does multi-turn assembly look (Router probe → tool exec → Final), and where does artifact aggregation need to hook? | Theater loop in `_run_theater_stream` (`agent.py:1087-1364`): each turn does (a) probe via `_router_client.generate_raw_text`, (b) `_parse_tool_calls`, (c) if calls, run `_execute_tool_calls` collecting `executed_results: list[ExecutedToolResult]`, (d) `_wash_tool_results`, (e) append `assistant` (cmd_only) + `user` (rendered tool results) to `self.messages`, (f) `continue` outer loop. If no calls, fall through to Final: `_final_persona_system_content` → buffered `generate_raw_text` → chunked `bb-stream-chunk` → `return`. **`executed_results` is per-turn-local — it is NOT accumulated across turns at any existing instance attribute**. Aggregation hook for FC.2: introduce a new instance attribute (e.g., `self._session_artifacts: list[Artifact]`) initialized in `__init__` (`agent.py:382-431`), extended after each `_wash_tool_results` returns (`agent.py:1213`), deduplicated, then emitted as a single `bb-message-artifacts` SSE frame at the **last yield site before `return`** in `_run_theater_stream` (`agent.py:1355-1356`). Rust must add a new branch in `stream_oligo_agent`'s event match (`llm_client.rs:294-324`) to forward it as a Tauri event before the success-path `"DONE"` emitted at `lib.rs:979`. | `agent.py:1087-1264` (turn body); `agent.py:1199` (`executed_results` per-turn local); `agent.py:1213-1215` (post-wash result list); `agent.py:1257-1262` (folding into messages); `agent.py:1294-1356` (final stream then return); `agent.py:382-431` (`__init__` — accumulator init site); `llm_client.rs:294-324` (event dispatch — extension site); `lib.rs:979` (success-path DONE — must follow artifacts emit) | Med |

---

## Cross-references discovered

- **`ExecutedToolResult` field set:** `call_id, tool_name, args, status, raw_result, washed_result, error_message, elapsed_ms` — constructed at `agent.py:706-716, 758-792, 824-835`. **No `artifacts` field today.** FC.1 spec says "extension only"; the natural extension is an optional `artifacts: list[Artifact] | None = None` parameter on `ExecutedToolResult` plus identical optional on `ToolOutput`.
- **`bb-tool-start` / `bb-tool-done` event JSON shape:** `{call_id, tool_name, started_at_ms}` and `{call_id, status, elapsed_ms}`. Established precedent for new Tauri-forwarded events. (`agent.py:735-743, 799-810, 841-852`)
- **Tool whitelist + denial:** Already enforced in `_parse_tool_calls` (`agent.py:602-618`); FC.1 must not change this.
- **Wash bypass list:** `agent_config.bypass_wash_tools` / `force_wash_tools` controls which tools' outputs are LLM-compressed (`agent.py:1011-1017`). FC.1 must verify whether structured `ToolOutput.text` should still pass through wash — note that `washed_result` becomes `result_body` at render time.
- **Vault path validation:** No central "is this path inside vault_root?" helper exists in vault_tools.py. FC.3's `open_vault_note` boundary check (red line) needs a new helper or inline check using `VaultReadAdapter`.

---

## Notable cross-findings (no fix proposals — flagging for planning)

1. **FC.5 (message delete) is materially already implemented.** `delete_chat_message` Tauri command, frontend `deleteMessage`, JSONL persistence on delete, and stage-card-non-deletability invariant are all in place at the cited lines. Whatever sprint plan FC.5 produces should start by re-verifying these against the FC.5 acceptance criteria — there may be little or nothing to build, only test/audit.

2. **FC.4 (persona/router separation) is largely already enforced** on the Oligo path. The Router stage actively warns-and-drops any component whose template contains `{persona}`. The remaining gap is a unit test asserting persona-invariance of the Router probe — which matches the FC.4 sprint's stated acceptance ("identical messages + different persona → identical Router probe output").

3. **Backend file paths in `docs/phases/Phase-III.C.md` are stale.** The sprint doc (lines 30–37, 47–61, etc.) references `src/oligo/...` and `src/crucible/...`, but the actual layout is `crucible_core/src/oligo/...` and `crucible_core/src/crucible/...`. This affects FC.1/FC.2/FC.4 file lists at planning time.

4. **`bb-stream-done` ownership is split between Python and Rust.** Python emits it only on error; Rust emits the success-path `"DONE"`. Any "before bb-stream-done" requirement (FC.2 red line) must therefore land its work **inside Python's last yield before `return`** AND **inside Rust's event handler before its `Ok(Some(...))` return path** — a single hook is insufficient.

5. **`_VaultToolPort.query_graph` adapter already returns `list[dict]`.** When FC.1 designs `Artifact`, the adapter layer can supply structured node data without a new code path; the lossy step today is `obsidian_graph_query`'s string-flattening at `vault_tools.py:154-167`.

---

**Audit complete.** 8 questions answered (Q1 with sub-question), 60+ file:line references, 5 cross-references and 5 notable cross-findings recorded. No fix proposals included.

**Suggested next:** `sprint_planning` for FC.1 (genuinely new opt-in `ToolOutput`; clean slate) — or bring FC.5 forward as a verify-only sprint given finding #1 and FC.4 as a test-only sprint given finding #2, deferring FC.1/FC.2/FC.3 to be the substantive work of the phase.
