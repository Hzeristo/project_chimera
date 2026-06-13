# IV.A.0 — Theater Loop Audit

**Sprint:** A.0  
**Date:** 2026-06-13  
**Files read:** `crucible_core/src/oligo/core/agent.py`, `crucible_core/src/crucible/services/task_service.py`, `crucible_core/src/oligo/core/sse.py`

---

## Q1 — Implicit States (what they ARE, not what they should be)

| State (inferred name) | Code region | "I'm in this state" invariant | Exit condition | Data produced / consumed |
|---|---|---|---|---|
| **PRE_TURN_SETUP** | `agent.py:1156–1160` | `turn > 1`; about to swap system slot | `messages[0]` replaced with `router_continuation_prompt` | Mutates `self.messages[0]`; no new data |
| **ROUTING** | `agent.py:1162–1198` | `self.messages` holds router system + sanitized history; no tool results from this turn yet | `probe_response` received from `_router_client`; `planned_calls` parsed | Produces `probe_response: str`, `planned_calls: list[PlannedToolCall]` |
| **EXECUTING** | `agent.py:1244–1279` | `planned_calls` non-empty; `wash_context` captured; `_tool_task` running; SSE queue drain in progress | `_tool_task` completes + queue sentinel (`None`) received | Consumes `planned_calls`; produces `executed_results: list[ExecutedToolResult]` with `raw_result` set |
| **WASHING** | `agent.py:1281–1307` | `executed_results` available with `raw_result`; LLM wash client may be called per tool | `_wash_tool_results` coroutine returns | Consumes `list[ExecutedToolResult]` (raw); produces same list with `washed_result` set + `wash_events: list[tuple[str,int]]` |
| **RENDERING_TOOL_RESULTS** | `agent.py:1309–1334` | `executed_results` washed; about to mutate `self.messages` | Two `ChatMessage` entries appended; `continue` fires | Appends `cmd_only` (assistant) and `tool_result_message` (user) to `self.messages`; loop restarts |
| **PROBE_DRAFT_BACKFILL** | `agent.py:1336–1362` | No tool calls in this turn; DSL stripped from probe; triviality decided | Optional assistant draft appended; falls through to SYNTHESIZING | Optionally appends `backfill_draft` to `self.messages` |
| **SYNTHESIZING** | `agent.py:1364–1439` | No tool calls this turn; final persona system built; `full_response` generated blocking | `full_response` chunked and yielded; `return` exits loop | Consumes `self.messages` + `final_system`; produces SSE stream to client + optional `bb-message-artifacts` frame |
| **TURN_EXHAUSTED (fallback)** | `agent.py:1441–1446` | `turn >= max_turns` and loop exits without `return` | Single `bb-stream-done` with error | No state mutation; terminal |

**Two states not named in the prompt:**
- **PRE_TURN_SETUP** (`agent.py:1156–1160`): Only exists on `turn > 1`. Silently replaces `messages[0]` with `router_continuation_prompt`. There is no flag or variable marking this happened — it is fully implicit.
- **PROBE_DRAFT_BACKFILL** (`agent.py:1336–1362`): A micro-state between ROUTING exit (no-tool) and SYNTHESIZING entry. The `_probe_trivial` boolean is the only marker; it lives as a local variable and is not visible to any other state.

---

## Q2 — Blocking awaits

| Location | What blocks | Necessary or Convertible | Notes |
|---|---|---|---|
| `agent.py:1177–1180` | `await asyncio.wait_for(self._router_client.generate_raw_text(api_messages), 120s)` | **NECESSARY** | Must have probe result to decide tool dispatch vs. synthesize. No forward progress without it. |
| `agent.py:1261–1265` | `await tool_sse_q.get()` — per-item drain of the tool SSE queue | Necessary for forwarding, not for execution | Execution is already in a `Task` (`_tool_task`). This drain is the correct pattern; agent is not blocked from yielding SSE while tools run. |
| `agent.py:1267` | `await _tool_task` — collect `executed_results` | **NECESSARY today; CONVERTIBLE for long_running tools** | For regular tools, must block to get results. For `long_running` tools (e.g. `daily_paper_pipeline`), the tool returns immediately with a `task_id` string — the agent could suspend here (AWAITING_TASK) instead of proceeding with a `task_id` as if it were a real result. |
| `agent.py:1281` → `agent.py:964` | `await compress_client.generate_raw_text(wash_messages)` inside `_wash_tool_results` | **NECESSARY before RENDERING** | Wash result must be available before injecting into `self.messages`. Could theoretically parallelize across tools but not across states. |
| `agent.py:1383–1388` | `await asyncio.wait_for(self.llm_client.generate_raw_text(...), 120s)` — Final generate | **NECESSARY** | Must buffer full response before simulated chunking. |
| `agent.py:1423` | `await asyncio.sleep(0.04)` — per-chunk pacing | **Not a blocking concern** | Artificial streaming pacing only; not a responsiveness issue. |

**Long-running task (AWAITING_TASK gap summary):** `daily_paper_pipeline` executes via `_execute_tool_with_deadline` → tool function schedules work in `TaskService` and returns `"Task started: {task_id}"` immediately. Agent's `wait_for` deadline is for the tool _function_ returning, which it does immediately. Agent proceeds normally with `task_id` string as result. There is no suspend point. The agent does not await task completion before entering SYNTHESIZING.

---

## Q3 — Data flows between states

| Transition | Data passed | Current type | Notes |
|---|---|---|---|
| **ROUTING → EXECUTING** | `planned_calls` | `list[PlannedToolCall]` | Typed. Defined at `agent.py:1195`, consumed at `agent.py:1251–1253`. `wash_context: str` also captured at `agent.py:1244` (consumed in WASHING). |
| **EXECUTING → WASHING** | `executed_results` | `list[ExecutedToolResult]` | Typed. `raw_result` set; `washed_result=None`. `agent.py:1267 → 1281`. |
| **WASHING → RENDERING_TOOL_RESULTS** | `executed_results` (washed) | `list[ExecutedToolResult]` | Typed. `washed_result` set. `agent.py:1281 return → 1285/1327`. Also produces `wash_events: list[tuple[str,int]]` for telemetry only. |
| **RENDERING_TOOL_RESULTS → ROUTING (next turn)** | Two new `ChatMessage` entries in `self.messages` | `ChatMessage(role="assistant", content=cmd_only)` + `ChatMessage(role="user", content=tool_result_message)` | Typed. `agent.py:1318–1330`. Loop state encoded entirely in `self.messages` shape — no explicit "I just ran tools" flag. |
| **ROUTING → PROBE_DRAFT_BACKFILL** | `probe_response: str` | `str` | Typed (unboxed). `_probe_trivial: bool` local flag. `agent.py:1336–1339`. |
| **PROBE_DRAFT_BACKFILL → SYNTHESIZING** | Optional `backfill_draft` appended to `self.messages`; no explicit handoff object | `ChatMessage(role="assistant", content=backfill_draft)` | The no-tool branch has no typed container summarizing "what the probe produced." Information lives in `self.messages` mutation. |
| **ROUTING → SYNTHESIZING (trivial/PASS probe)** | Nothing explicit — loop falls through | — | Router probe result is discarded if trivial. `self.messages` unchanged. |
| **SYNTHESIZING → (terminal)** | SSE stream yielded to caller | `str` frames (chunked) | No state mutation after `return`; `_session_artifacts` emitted as `bb-message-artifacts` if non-empty. |

**Cross-state locals that carry data but are not typed containers:**
- `wash_context: str` — captured at ROUTING exit (`agent.py:1244`), consumed in WASHING (`agent.py:1281`). Spans EXECUTING without being passed explicitly.
- `probe_for_cmd: str` — a re-strip of `probe_response` used at `agent.py:1309` for history backfill. Lives as a local across the tool execution block.

---

## Q4 — AWAITING_TASK gap

| Question | Answer | Code evidence |
|---|---|---|
| What currently happens when `daily_paper_pipeline` returns `task_id`? | Tool function returns immediately with a string (e.g. `"Task started: {task_id}"`). Agent's `wait_for` deadline applies to the tool _function_ returning — which it does immediately. `executed_results` contains `raw_result = "Task started: …"`. Loop proceeds: WASHING (bypass or no-op on short string) → RENDERING_TOOL_RESULTS → `continue` → ROUTING (next turn). Router sees `[SYSTEM TOOL RESULTS]` with `task_id`, no await instruction → PASS → SYNTHESIZING fabricates completion. | `agent.py:678–697` (deadline wrapper); `agent.py:1267–1334` (no suspend); `task_service.py:301–313` (`run_task` takes an `Awaitable` but is called from the tool, not the agent) |
| Where should the suspend point be? | After EXECUTING resolves `executed_results`: detect that any result came from a `long_running` tool and contains a `task_id`. At that point, instead of flowing to WASHING, the agent should enter AWAITING_TASK and yield control. | No suspend point exists today. `ToolSpec.long_running` flag is defined in the registry but never checked in `_run_theater_stream`. |
| What event does the resume need? | `TaskEvent` with `event_type == TaskEventType.COMPLETED` (or `FAILED`) for the matching `task_id`. `task_service.py:190–212` shows `emit_completed` sets `task.result` and fires the event. | `task_service.py:97–107`: `emit_event` pushes to `self._event_queue` and all `self._subscribers`. `subscribe()` returns an `asyncio.Queue[TaskEvent]` — the infrastructure for resume already exists. |
| After resume, which state? | Agent should re-enter with the real `task.result` as the `washed_result` of the original `ExecutedToolResult`, then flow to RENDERING_TOOL_RESULTS → ROUTING (next turn). The resume needs to construct a synthetic `ExecutedToolResult(status=COMPLETED, washed_result=task.result)` for the `task_id` it was waiting on. | No code today. |

---

## Q5 — Turn-boundary state (what persists across `while` iterations beyond `self.messages`)

| Persisted item | Type | Where set | What it encodes |
|---|---|---|---|
| `turn` (local) | `int` | `agent.py:1149` init, `1152` increment | Iteration counter. Also mirrored as `self._current_turn` (`agent.py:1153`). |
| `self._current_turn` | `int` | `agent.py:1153` | Exposed to `fork_subagent` for budget computation (`agent.py:1475`). |
| `self._session_artifacts` | `list[Artifact]` | `agent.py:429` init; `agent.py:1285` populated via `_accumulate_artifacts` | Cross-turn artifact accumulator; emitted once at SYNTHESIZING exit. Survives across tool-bearing turns. |
| `self._artifact_keys` | `set[tuple[str,str]]` | `agent.py:430` init; `agent.py:999` populated | Dedup set for `_session_artifacts`. Same lifecycle. |

**No explicit "why are we looping" context.** The loop has no variable that says "last turn dispatched tool X; I am continuing because of that." The only signal is the shape of `self.messages`: if the last two messages are `role=assistant` (cmd_only) + `role=user` (`[SYSTEM TOOL RESULTS]`), we are in a tool-continuation turn. This is inferred from message content, not from typed state.

**Implication for FSM design:** An AWAITING_TASK state would require a carried context field — at minimum `task_id: str` and `pending_result: ExecutedToolResult` — because nothing in `self.messages` or the existing persisted fields encodes "I am suspended waiting for task X."

---

## Summary findings for user's schema design

1. Five main states confirmed (`ROUTING`, `EXECUTING`, `WASHING`, `RENDERING_TOOL_RESULTS`, `SYNTHESIZING`); two additional micro-states found (`PRE_TURN_SETUP`, `PROBE_DRAFT_BACKFILL`).
2. All inter-state data is currently typed (`PlannedToolCall`, `ExecutedToolResult`, `ChatMessage`) — the domain objects already exist. The FSM needs to name the transitions, not invent new types.
3. The only CONVERTIBLE block is `await _tool_task` for `long_running` tools. All other major blocks are NECESSARY.
4. `TaskService.subscribe()` → `asyncio.Queue[TaskEvent]` is already wired. Resume from AWAITING_TASK is implementable by subscribing at suspend time and `await`-ing the matching `COMPLETED`/`FAILED` event.
5. Turn-boundary state is minimal. FSM carried-context would need to add `task_id` + `suspended_result` for AWAITING_TASK; nothing else is missing.

---

*Do NOT read this as a proposed schema. State names above are labels for audit purposes only. Schema design (A.1) is the user's call after reading this.*
