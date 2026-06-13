# Batch Plan: Phase IV.A — Async Agent Core (Sprints A.3–A.6)

**Output location:** `docs/plans/Phase-IV.A-batch.md`
**Audit reference:** `docs/audits/IV.A.0.md`
**Phase doc:** `docs/phases/phase-IV.A.md`
**Sealed predecessors:** A.0 (audit), A.1 (identity DDD), A.2 (phase labels + narrow results)
**Driving friction:** friction-260611/260613 E1 — long-running task → Final fabricates; no await mechanism

Approved schema (A.2): `AgentPhase`, `RouteResult`, `ExecuteResult`, `TerminalReason`, `TurnOutcome`
in `crucible_core/src/oligo/core/schemas.py`.

This document is a single unit. User approves the whole sequence or rejects it.
After approval, hand off to `chimera-code-taste` batch_execution mode.

---

## Sprint Sequence

```
A.3 (coroutine refactor) → A.4 (AWAITING_TASK) → A.5 (SSE protocol) → A.6 (lifecycle)
```

Linear. A.3 must be behaviorally sealed before A.4 adds suspension — the HSC
"byte-identical SSE stream" verifies this boundary. A.5 and A.6 both depend on
the AWAITING_TASK machinery from A.4.

---

## Sprint A.3 — Coroutine Refactor (PURE REFACTOR)

**Predecessor assumptions:**
- A.2 sealed: `AgentPhase`, `RouteResult`, `ExecuteResult` present in schemas.py
- `self._phase: AgentPhase = AgentPhase.ROUTING` initialized in `ChimeraAgent.__init__`

**Risk level:** 🟡 MED — large structural change to `_run_theater_stream`; zero behavior change intended.
All risk is in the HSC: byte-identical SSE output is the regression gate.

### Goal

Decompose `_run_theater_stream` (currently one 300-line while-loop) into five async step methods.
Data flows via args/returns using the A.2 result objects. `self._phase` updated per step.
**No new capabilities. No new awaits. No behavioral change.**

### Audit basis

- `IV.A.0.md:Q1` — five main states identified with exact code regions
- `IV.A.0.md:Q3` — cross-state locals `wash_context` (captured `agent.py:1244`, consumed `agent.py:1281`)
  and `probe_for_cmd` (captured `agent.py:1309`) are the main threading problem; both become
  explicit `RouteResult` fields (already defined in A.2)
- `IV.A.0.md:Q2` — all five major `await` points confirmed NECESSARY; none removed in A.3

### Step method signatures

```python
async def _step_route(self) -> RouteResult:
    # audit: agent.py:1162–1213 (ROUTING + PRE_TURN_SETUP)
    # returns RouteResult(probe_response, planned_calls, wash_context, probe_for_cmd, is_trivial)

async def _step_execute(
    self,
    planned_calls: list[PlannedToolCall],
    emit_tool_sse: Callable[[str], Awaitable[None]] | None,
) -> ExecuteResult:
    # audit: agent.py:1215–1282 (EXECUTING)
    # returns ExecuteResult(executed_results, has_long_running=False for A.3)

async def _step_wash(
    self,
    executed_results: list[ExecutedToolResult],
    wash_context: str,
) -> list[ExecutedToolResult]:
    # audit: agent.py:1283–1308 (WASHING)
    # returns washed list; also emits wash SSE telemetry (side-effect, not in return)

def _step_render(
    self,
    executed_results: list[ExecutedToolResult],
    probe_for_cmd: str,
    turn_id: str,
) -> None:
    # audit: agent.py:1309–1334 (RENDERING_TOOL_RESULTS)
    # mutates self.messages (appends cmd_only + tool_result_message); returns nothing

async def _step_synthesize(
    self,
    probe_response: str,
    is_trivial: bool,
    turn_id: str,
) -> AsyncGenerator[str, None]:
    # audit: agent.py:1336–1454 (PROBE_DRAFT_BACKFILL + SYNTHESIZING)
    # yields SSE chunks; returns after final bb-message-artifacts frame
```

### Refactored `_run_theater_stream` skeleton

```python
async def _run_theater_stream(self) -> AsyncGenerator[str, None]:
    turn = 0
    while turn < self.max_turns:
        turn += 1
        self._current_turn = turn
        _turn_ctx = TurnContext(
            turn_id=TurnId.create(self._conversation_ctx.session_id, turn),
            turn_number=turn,
        )
        turn_id_str = str(_turn_ctx.turn_id)

        self._phase = AgentPhase.ROUTING
        route = await self._step_route()

        if route.planned_calls:
            tool_sse_q: asyncio.Queue[str | None] = asyncio.Queue()
            async def emit_tool_sse(frame: str) -> None:
                await tool_sse_q.put(frame)

            self._phase = AgentPhase.EXECUTING
            execute_task = asyncio.create_task(
                self._step_execute(route.planned_calls, emit_tool_sse)
            )
            while True:
                item = await tool_sse_q.get()
                if item is None:
                    break
                yield item
            execute_result = await execute_task

            self._phase = AgentPhase.WASHING
            washed = await self._step_wash(execute_result.executed_results, route.wash_context)
            self._phase = AgentPhase.RENDERING
            self._step_render(washed, route.probe_for_cmd or "", turn_id_str)
            continue

        self._phase = AgentPhase.SYNTHESIZING
        async for chunk in self._step_synthesize(route.probe_response, route.is_trivial, turn_id_str):
            yield chunk
        return

    yield sse_event("bb-stream-done", {"error": True, "message": "[SYSTEM FATAL]: Agent exhausted max turns."})
```

### Scope

| File | Change |
|---|---|
| `agent.py` | Extract `_step_route`, `_step_execute`, `_step_wash`, `_step_render`, `_step_synthesize` from while-loop body; replace loop body with skeleton above; `self._phase =` assigned at each step boundary in the main loop (plain assignment, no SSE) |
| `schemas.py` | No change (A.2 types already present) |

Estimated lines touched: `agent.py:1157–1461` (~305 lines restructured, no net addition).

### Hard Sealing Conditions

1. **Byte-identical SSE stream**: run the existing test suite (`pytest tests/oligo/`) — 0 new failures.
   Additionally: same mock conversation through old vs. new loop produces identical `"".join(chunks)`.
2. **Phase label wired**: `self._phase` assigned at each step boundary in the main loop (plain
   assignment, no SSE). `grep` confirms NO `self._phase` assignment inside any `_step_*` method body.
   No `bb-phase-transition` emitted in A.3 — SSE stream byte-identical to pre-refactor.
3. **No StateContext**: `grep -r "StateContext\|StateMachine" crucible_core/src/oligo/` — zero matches.
4. **No cross-state locals**: `grep -n "wash_context\|probe_for_cmd" agent.py` — only appears as
   `RouteResult` field access or method parameter, never as bare closure local spanning methods.

### Red Lines

- ❌ NO behavioral change — `_step_synthesize` must yield the exact same SSE chunk sequence as the old SYNTHESIZING block
- ❌ NO new `await` expressions beyond those in the original code — suspension is A.4's scope
- ❌ `_step_render` is synchronous — it only mutates `self.messages`; do NOT make it async
- ❌ `has_long_running` in `ExecuteResult` stays `False` for all A.3 paths — A.4 sets it
- ❌ Do NOT move `self._session_artifacts` or `self._artifact_keys` — they stay agent-level, populated inside `_step_wash` via `_accumulate_artifacts`

---

## Sprint A.4 — AWAITING_TASK via Coroutine Suspension

**Predecessor assumptions:**
- A.3 sealed and HSC passed: byte-identical SSE confirmed, step methods in place

**Risk level:** 🔴 HIGH — first genuine behavior change; fixes the E1 root cause.
New code path (`AWAITING_TASK`) never executed before; must not disturb the non-long-running path.

### Goal

Long-running tools (`ToolSpec.long_running=True`: `arxiv_miner`, `daily_paper_pipeline`) currently
return `"Task started: {task_id}"` immediately and the agent fabricates results. After A.4:
the agent detects the `task_id` result, enters `AWAITING_TASK`, suspends via `await`, and resumes
with real results when `TaskService` fires a `COMPLETED`/`FAILED` event.

This is the E1 fix. Audit finding: `IV.A.0.md:Q4`.

### Audit basis

- `IV.A.0.md:Q4` — `TaskService.subscribe()` returns `asyncio.Queue[TaskEvent]` (task_service.py:109–113).
  `emit_completed` fires `TaskEventType.COMPLETED` with `message=task.result` (task_service.py:200–212).
  Infrastructure for event-driven resume already exists.
- `IV.A.0.md:Q4` — suspend point: after `_step_execute` resolves, detect `ExecuteResult.has_long_running=True`.
- `IV.A.0.md:Q5` — nothing in `self.messages` encodes suspension; carried context must live in
  coroutine-local variables (`task_id`, `pending_er`).
- `registry.py:274,311` — `long_running=True` on `arxiv_miner` and `daily_paper_pipeline`.

### Design

**Structured task_id — parse once at execution boundary** (in `_step_execute`): after each tool
returns, if `ToolSpec.long_running` is True and `raw_result` starts with `"Task started: "`, parse
the `task_id` exactly once and store it on the result object:

```python
# in _step_execute, after building each ExecutedToolResult:
if registry.is_long_running(tool_name) and (raw or "").startswith("Task started: "):
    er = er.model_copy(update={"task_id": raw.removeprefix("Task started: ").strip()})
```

Requires adding to `ExecutedToolResult` in `schemas.py`:
```python
task_id: str | None = Field(None, description="Set when a long_running tool spawned a background task")
```

All downstream detection uses the structured field — never string parsing:
```python
has_long_running = any(er.task_id is not None for er in executed_results)
```

**`TaskService` additions:**

`get_task(task_id) -> Task | None` — load persisted task (already `_load_task` internally; expose
as public method returning `None` on missing file rather than raising).

`_event_from_task(task: Task) -> TaskEvent` — synthesize a terminal `TaskEvent` from persisted
task state (used by the pre-subscribe double-check):
```python
def _event_from_task(self, task: Task) -> TaskEvent:
    etype = {
        TaskStatus.COMPLETED: TaskEventType.COMPLETED,
        TaskStatus.FAILED: TaskEventType.FAILED,
        TaskStatus.CANCELLED: TaskEventType.CANCELLED,
    }[task.status]
    return TaskEvent(
        event_type=etype, task_id=task.id, task_type=task.type,
        stage_id=None, stage_label=None, overall_progress=task.progress,
        message=task.result, error=task.error, timestamp_ms=self._now_ms(),
    )
```

**`TaskService.await_completion(task_id)`** — double-check pattern eliminates subscribe-after-emit
race (fast tasks completing before subscribe):

```python
async def await_completion(self, task_id: str, timeout_s: float = 600.0) -> TaskEvent:
    """Event-driven wait. Double-checks terminal status around subscribe
    to eliminate the subscribe-after-emit race (fast tasks finishing
    before we subscribe)."""
    # Check 1: already terminal before subscribing?
    task = self.get_task(task_id)
    if task and task.status in (TaskStatus.COMPLETED, TaskStatus.FAILED, TaskStatus.CANCELLED):
        return self._event_from_task(task)

    q = self.subscribe()
    try:
        # Check 2: terminal in the window between check 1 and subscribe?
        task = self.get_task(task_id)
        if task and task.status in (TaskStatus.COMPLETED, TaskStatus.FAILED, TaskStatus.CANCELLED):
            return self._event_from_task(task)

        deadline = asyncio.get_event_loop().time() + timeout_s
        while True:
            remaining = deadline - asyncio.get_event_loop().time()
            if remaining <= 0:
                raise asyncio.TimeoutError(f"await_completion timed out for {task_id}")
            event = await asyncio.wait_for(q.get(), timeout=remaining)
            if event.task_id == task_id and event.event_type in (
                TaskEventType.COMPLETED, TaskEventType.FAILED, TaskEventType.CANCELLED
            ):
                return event
    finally:
        self.unsubscribe(q)
```

**Strong task references** — `TaskService` must hold a strong ref to each running `asyncio.Task`
so the GC cannot collect it before completion. Confirm `_running_tasks: dict[str, asyncio.Task]`
exists; if not, add it. Every `create_task` call stores the ref and removes on done:
```python
t = asyncio.create_task(self.run_task(task_id, work))
self._running_tasks[task_id] = t
t.add_done_callback(lambda _t: self._running_tasks.pop(task_id, None))
```

**`_phase_event` helper** — introduced in A.4 (needed for AWAITING_TASK signal):
```python
def _phase_event(self, phase: AgentPhase) -> str:
    self._phase = phase
    return sse_event("bb-phase-transition", {
        "phase": phase.value,
        "turn": self._current_turn,
        "timestamp_ms": int(time.time() * 1000),
    })
```

In A.4 this is used **only** for `AWAITING_TASK`. The plain `self._phase =` assignments from
A.3 remain for the other phases; A.5 replaces them with `yield self._phase_event(...)`.

**Gather-all AWAITING_TASK block** in `_run_theater_stream` (after A.3 skeleton):

```python
if execute_result.has_long_running:
    yield self._phase_event(AgentPhase.AWAITING_TASK)
    svc = get_task_service()
    lr_ers = [er for er in execute_result.executed_results if er.task_id]
    events = await asyncio.gather(
        *[svc.await_completion(er.task_id) for er in lr_ers],
        return_exceptions=True,
    )
    event_by_call = {er.call_id: ev for er, ev in zip(lr_ers, events)}
    patched = []
    for er in execute_result.executed_results:
        ev = event_by_call.get(er.call_id)
        if ev is None:
            patched.append(er)
        elif isinstance(ev, BaseException):
            patched.append(er.model_copy(update={
                "status": ToolCallStatus.ERROR,
                "error_message": f"await failed: {ev}",
                "washed_result": f"[Task await failed] {ev}",
            }))
        elif ev.event_type == TaskEventType.COMPLETED:
            patched.append(er.model_copy(update={
                "washed_result": ev.message or "",
                "raw_result": ev.message or "",
                "status": ToolCallStatus.SUCCESS,
            }))
        else:  # FAILED / CANCELLED
            patched.append(er.model_copy(update={
                "status": ToolCallStatus.ERROR,
                "error_message": ev.error or "task failed",
                "washed_result": f"[Task failed] {ev.error or ''}",
            }))
    washed = await self._step_wash(patched, route.wash_context)
    self._step_render(washed, route.probe_for_cmd or "", turn_id_str)
    continue
```

### Scope

| File | Change |
|---|---|
| `task_service.py` | Add `await_completion` (~30 lines); add `get_task` public method; add `_event_from_task` helper; add `_running_tasks: dict` with `done_callback` cleanup |
| `agent.py` | Add `_phase_event` helper (~6 lines); add AWAITING_TASK gather-all branch in `_run_theater_stream` (~25 lines); `_step_execute` sets `er.task_id` at parse boundary and `ExecuteResult.has_long_running` |
| `schemas.py` | Add `task_id: str | None = Field(None, ...)` to `ExecutedToolResult` |

### Hard Sealing Conditions

1. **E1 fix**: `"爬取论文"` → final reply contains REAL paper IDs cross-referenced with `audit_log.csv`.
   Verified manually in live session.
2. **Event-driven, not polling**: `grep -n "sleep\|polling\|while.*task"` in the AWAITING_TASK branch
   — zero timed loops. `await svc.await_completion(task_id)` is the only wait.
3. **Non-long-running path unchanged**: mock conversation with no `long_running` tool produces
   byte-identical SSE to A.3 output (regression).
4. **TASK_FAILED surfaces**: when `emit_failed` fires for the task, final reply contains
   `[Task failed]` text, not fabricated results.
5. **Fast-task race**: a long-running tool that completes in <3s resolves correctly via the
   pre-subscribe check and does NOT block until timeout. Verified by a test that emits `COMPLETED`
   before `await_completion` is called.
6. **Two long-running tools**: both awaited via `asyncio.gather`, both results land in history,
   neither orphaned. Verified by mock emitting `COMPLETED` for both.
7. **Strong task refs**: background `asyncio.Task` objects stored in `_running_tasks`; no GC
   mid-flight. Verified by checking `len(svc._running_tasks)` is non-zero while task runs.

### Red Lines

- ❌ NO polling loop — resume MUST be via `TaskService.await_completion` (event queue, double-check)
- ❌ NO serialization of suspension state — if agent crashes mid-await, the turn is lost; do NOT attempt recovery
- ❌ All long-running tasks in a turn awaited via `asyncio.gather`; no background task is orphaned
- ❌ Do NOT skip `_step_wash` on the resumed results — wash policy applies to real results too
- ❌ `await_completion` timeout is 600s; do NOT reduce without a friction signal — paper pipeline takes 3–8 min
- ❌ `await_completion` MUST double-check terminal status before AND after subscribe (no subscribe-after-emit race)
- ❌ `task_id` is a structured `ExecutedToolResult` field; `"Task started: "` string parsed exactly once in `_step_execute`, never downstream
- ❌ NO fire-and-forget `asyncio.create_task` — every background task stored in `_running_tasks` (strong ref) with `done_callback` cleanup

---

## Sprint A.5 — SSE Protocol: Phase-Transition Events

**Predecessor assumptions:**
- A.4 sealed: AWAITING_TASK branch working, E1 HSC passed

**Risk level:** 🟢 LOW — additive only; existing `bb-stream-*` events untouched.

### Goal

Frontend learns the agent's current phase via new named SSE events emitted at each step entry.
Enables UI to show "Agent is searching…", "Agent is awaiting task…" etc. without polling.

### Design

`_phase_event` was introduced in A.4 for the `AWAITING_TASK` signal. A.5 extends it to the
remaining five phases by replacing the plain `self._phase =` assignments from A.3 with
`yield self._phase_event(...)` at each loop boundary.

A.4 loop (post-patch) has:
- `self._phase = AgentPhase.ROUTING` → already a plain assignment (A.3)
- `self._phase = AgentPhase.EXECUTING` → plain assignment (A.3)
- `self._phase = AgentPhase.WASHING` → plain assignment (A.3)
- `self._phase = AgentPhase.RENDERING` → plain assignment (A.3)
- `self._phase = AgentPhase.SYNTHESIZING` → plain assignment (A.3)
- `yield self._phase_event(AgentPhase.AWAITING_TASK)` → SSE-emitting (A.4)

A.5 converts the five plain assignments to SSE-emitting calls:

```python
yield self._phase_event(AgentPhase.ROUTING)
route = await self._step_route()

if route.planned_calls:
    yield self._phase_event(AgentPhase.EXECUTING)
    # ... execute block ...
    yield self._phase_event(AgentPhase.WASHING)
    washed = await self._step_wash(...)
    yield self._phase_event(AgentPhase.RENDERING)
    self._step_render(...)
    continue

yield self._phase_event(AgentPhase.SYNTHESIZING)
async for chunk in self._step_synthesize(...):
    yield chunk
return
```

The `AWAITING_TASK` call from A.4 remains unchanged — no duplication.

**Backward compatibility**: `bb-phase-transition` is a new event name. Existing handlers for
`bb-stream-chunk`, `bb-stream-done`, `bb-tool-start`, `bb-tool-done`, `bb-message-artifacts`,
`__SYS_TOOL_CALL__` are unaffected.

### Scope

| File | Change |
|---|---|
| `agent.py` | Replace 5 plain `self._phase =` assignments in the main loop with `yield self._phase_event(...)` (~5 line changes); `_phase_event` already present from A.4 |
| `sse.py` | No change |
| `schemas.py` | No change |

Estimated net change: 5 line substitutions.

### Hard Sealing Conditions

1. **Phase visible**: `bb-phase-transition` events appear in SSE stream for all 6 phases in a live
   `"爬取论文"` session. Verified by reading raw SSE output.
2. **Backward compat**: existing `bb-stream-chunk` / `bb-stream-done` / `bb-tool-*` sequence
   unchanged when `bb-phase-transition` frames are filtered out. The `awaiting_task` phase event
   from A.4 is NOT duplicated — exactly one `bb-phase-transition{"phase":"awaiting_task"}` per turn.
3. **AWAITING_TASK phase visible**: during a long-task session, `bb-phase-transition {"phase": "awaiting_task"}`
   appears in the SSE stream before the task result arrives.

### Red Lines

- ❌ Do NOT change the payload shape of existing `bb-stream-*` events
- ❌ Do NOT emit `bb-phase-transition` inside tool execution (between `bb-tool-start` and `bb-tool-done`) — only at step boundaries
- ❌ Phase enum has no failure variants — `bb-phase-transition` never carries `TURN_EXHAUSTED` or similar
- ❌ Exactly ONE awaiting signal — `bb-phase-transition{awaiting_task}` from A.4; do NOT add a second awaiting telemetry frame

---

## Sprint A.6 — Lifecycle Integrity

**Predecessor assumptions:**
- A.5 sealed: phase events wired, AWAITING_TASK path confirmed working end-to-end

**Risk level:** 🟡 MED — tests an edge case (post-await continuation) that has never been exercised.
No new code expected; this sprint is verification + any fixup needed.

### Goal

After a long-running task completes and the final reply streams, the **next user message** in the
same session must enter a clean ROUTING state: correct `self.messages` history (task result in
history, no suspended state), correct `self._current_turn` reset, `self._phase` back to ROUTING.

Audit basis: `IV.A.0.md:Q5` — `self.messages` is the only cross-turn state besides
`_current_turn`, `_session_artifacts`, `_artifact_keys`. All of these must be clean after
a full AWAITING_TASK → SYNTHESIZING → return cycle.

### What to verify

1. **Message history**: after a `daily_paper_pipeline` turn completes, `self.messages` contains
   the real task result as a `role=user` tool-result message (not `"Task started: …"`).
   The next call to `_run_theater_stream` (new turn, new request) sees that history correctly.

2. **Turn counter**: `self._current_turn` is reset to 0 at the start of each `run_theater` call
   (it is set inside `_run_theater_stream`'s while-loop, not in `__init__` — verify).
   If it is NOT reset, add reset at top of `_run_theater_stream`.

3. **Artifacts**: `self._session_artifacts` and `self._artifact_keys` are per-request (initialized
   in `__init__`). Since `ChimeraAgent` is instantiated per-request (audit: confirmed by
   `agent.py:384`), these are clean by construction. Verify in integration test.

4. **Phase label**: after `run_theater` returns, `self._phase` is `AgentPhase.SYNTHESIZING`
   (last step executed). This is fine — `self._phase` is per-agent-instance, not cross-request.

### Design

No new code expected. This sprint runs a structured lifecycle scenario and fixes any issues found.

Scenario:
```
Request 1: user asks "爬取论文 agent memory 5篇"
  → ROUTING → EXECUTING → AWAITING_TASK (suspend) → resume → WASHING → RENDERING → ROUTING (next turn)
  → ROUTING → SYNTHESIZING → return
  → verify: self.messages[-2] is role=assistant (cmd_only), self.messages[-1] is role=user (real results)

Request 2 (new ChimeraAgent instance, same session_id, history threaded): user asks "总结一下"
  → verify: ROUTING probe sees the real paper result in history, not "Task started: …"
  → verify: no leftover AWAITING_TASK state, no task subscription leak
```

### Scope

| File | Change |
|---|---|
| `agent.py` | Fixup only — reset `self._current_turn = 0` at start of `_run_theater_stream` if not already (check `agent.py:1157`); ensure `await_completion` subscription is always cleaned up (finally block in `await_completion` already handles this) |
| `task_service.py` | Verify `unsubscribe` called in all paths of `await_completion` (already in finally block) |
| `tests/oligo/` | Add `test_lifecycle_after_long_task.py` — mock `TaskService`, simulate COMPLETED event, verify message history on second request |

### Hard Sealing Conditions

1. **History correct**: after a long-task turn, `self.messages` last tool-result message contains
   real task output, not `"Task started: {task_id}"`.
2. **No subscription leak**: `grep` on subscriber set size after task completion — 0 lingering
   subscribers for the completed task's queue.
3. **Clean next turn**: second request in same session produces a correct ROUTING probe that
   references real task results (verified via mock LLM asserting the history it received).

### Red Lines

- ❌ Do NOT add persistent suspension recovery — if the agent crashes mid-await, the turn is lost; that is an accepted limitation (phase doc Design Decisions)
- ❌ Do NOT change `ChimeraAgent` to be a long-lived stateful object shared across requests — it is and stays per-request
- ❌ Do NOT add a session cache or cross-request state store — out of scope (Phase V+)

---

## Phase-Wide Red Lines

- ❌ NO StateContext god-object — data flows via step args/returns only
- ❌ NO explicit transition table or StateMachine class — coroutine flow IS the machine
- ❌ NO reified FSM — `self._phase` is observation only, never drives control flow
- ❌ NO concurrent turns — AWAITING_TASK suspends ONE turn; parallel turns are Phase VI+
- ❌ NO heavyweight framework (LangGraph, Temporal, Celery)
- ❌ A.3 is a PURE REFACTOR — `has_long_running` stays False, no suspension added until A.4
- ❌ SSE backward compat — existing `bb-stream-*` events unchanged through A.5
- ❌ `await_completion` MUST double-check terminal status before AND after subscribe (no subscribe-after-emit race)
- ❌ `task_id` is a structured `ExecutedToolResult` field; `"Task started: "` string parsed exactly once in `_step_execute`, never downstream
- ❌ All long-running tasks awaited via `asyncio.gather`; zero orphaned background tasks
- ❌ Every background `asyncio.Task` strong-referenced in `_running_tasks` with `done_callback` cleanup
- ❌ Exactly one awaiting SSE signal (`bb-phase-transition{awaiting_task}` from A.4)

---

## Hard Sealing Conditions (Phase-level summary)

| # | Sprint | Condition |
|---|---|---|
| HSC-1 | A.3 | Byte-identical SSE stream before/after coroutine refactor (pytest + manual diff) |
| HSC-2 | A.3 | `self._phase` assigned at each step boundary in the main loop (plain assignment). `grep` confirms NO `self._phase` inside any `_step_*` body. No `bb-phase-transition` emitted in A.3. |
| HSC-3 | A.4 | `"爬取论文"` → real paper IDs in final reply, verified against audit_log.csv |
| HSC-4 | A.4 | No timed polling loop in AWAITING_TASK resume path — pure event-driven |
| HSC-5 | A.5 | `bb-phase-transition` visible in SSE stream for all 6 phases; existing event sequence unchanged |
| HSC-6 | A.6 | Post-await message history contains real results; no subscription leak |
| HSC-7 | A.4 | Fast task (<3s) resolves via pre-subscribe check, not timeout |
| HSC-8 | A.4 | Two long-running tools: both awaited via gather, neither orphaned |
| HSC-9 | A.4 | Background tasks strong-referenced in `_running_tasks`; no GC mid-flight |

---

## Approval

User approves whole sequence or rejects whole sequence.

Upon approval, hand off to `chimera-code-taste` with:
> "Execute batch for Phase IV.A (A.3–A.6) per `docs/plans/Phase-IV.A-batch.md`."

---

*Generated by chimera-sprint-discipline batch_planning mode — 2026-06-13.*
