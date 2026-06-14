# Phase IV.A — Async Agent Core

**Status:** Sealed 2026-06-14
**Sealed predecessor:** III.F
**Driving frictions:**
- friction-260611 / 260613 E1 (long-running task → Final fabricates,
  no await mechanism) — RECURRENT, now architectural
- The synchronous ReAct loop cannot model: task await, mid-turn
  suspension, event-driven continuation, or concurrent independent work.

## Mission

Refactor Oligo's synchronous ReAct theater loop into an explicit async
state machine. This is the relocation from "ReAct toy" to "usable agent
runtime." It establishes the concrete DDD + stable schema on which all
Phase V features (nodes, deep_research, Claude Code bridge) will build.
No new user-facing features — this phase rebuilds the foundation.

## Why before Phase V (not after)

The async state machine requires concrete DDD and stable schema. Every
Phase V feature (node ontology, deep_research, Claude Code bridge) depends
on the agent's execution model. Building features on a synchronous loop
then retrofitting async = building on quicksand. State machine first.

## Why Identity + Process DDD before async

Audit IV.A.0 confirms domain data layer (`PlannedToolCall`, `ExecutedToolResult`,
`ChatMessage`) is fully typed. But process orchestration is encoded in:
- message-list shape (last two messages = "we ran tools last turn")
- closure variables (`wash_context` lives across EXECUTING without being passed)
- absence of `TurnId` / explicit transition objects

This is "half-objectified" — Layer 1 (data) done, Layer 3 (process) missing.
Adding async on top of half-objectified process produces async closures, not
async state machines.

The execution model is coroutine-native, NOT a reified state machine.
Python's async/await already compiles each async function into a state
machine; each `await` is a suspension point. Building an explicit FSM
(state enum + transition table + serializable context) on top of
coroutines re-implements what the language provides for free.

Rejected: a StateContext god-object carrying all step inputs/outputs.
That is a blackboard wearing a state-machine costume — every step reads
and writes a shared mutable bag, zero encapsulation. Data flows via
function args/returns and narrow result objects instead.

Reified FSM would only be justified by (a) cross-process suspension
persistence, (b) transition replay, or (c) highly-branched control flow.
Chimera needs none: single-process, near-linear flow (only branch is
tool-call vs no-tool), sessions recoverable from jsonl on restart.

Time, not data, is the async concern. AWAITING_TASK is a *temporal*
state (waiting on an event over time), modeled by `await`-ing a
TaskService completion event — coroutine-native suspension, event wakeup,
free event loop during the wait. No busy-poll, no message queue.

## Sprint Sequence

| Sprint | One-line goal | Status |
|---|---|---|
| A.0 | Audit current theater loop, identify implicit states, blocking awaits, data flows, AWAITING_TASK gap | Sealed |
| A.1 | Identity DDD (Layer 2): TurnId + explicit Conversation/Turn context objects; threaded through ChatMessage, ExecutedToolResult, etc. Schema-only, no executor change | Sealed |
| A.2 | Phase labels + narrow result objects + TerminalReason enum. NO StateContext god-object, NO StateMachine class, NO transition table. Thin introspection layer only | Sealed |
| A.3 | Coroutine refactor: decompose _run_theater_stream into async step methods (route/execute/wash/render/synthesize); data via args/returns; self._phase label updated per step. PURE REFACTOR — byte-identical behavior | Sealed |
| A.4 | AWAITING_TASK via coroutine suspension: implement TaskService.await_completion(task_id); long_running tools `await` it; real result re-enters the flow. Event-driven by construction (not polling) | Sealed |
| A.5 | SSE protocol upgrade: state-transition events streamed mid-turn (frontend learns when agent is awaiting) | Sealed |
| A.6 | Lifecycle integrity: agent processes next user message correctly after a long-task await completes | Sealed |

Dependencies: A.0 → A.1 (identity schema) → A.2 (process schema) →
A.3 (FSM wired, pure refactor) → A.4 (async on FSM) → A.5 (SSE reflects states) →
A.6 (lifecycle integrity).

## Cross-Sprint Red Lines

- ❌ NO heavyweight framework (no LangGraph, no Temporal, no Celery) —
  hand-rolled state machine, asyncio primitives only
- ❌ A.3 is a PURE refactor — byte-identical observable behavior, just
  FSM-driven. Do NOT add await/event features in A.3 (those are A.4)
- ❌ State transitions MUST be explicit and typed — no implicit state in
  local variables; every state is a Pydantic model
- ❌ NO concurrent turns in this phase — async means "suspend/resume on
  events," NOT "parallel turns." Parallel turns are Phase V+ if ever
- ❌ Poison-pill / boundary markers (your video-pipeline pattern) allowed
  for clean shutdown, but NO global mutable state shared across the FSM
- ❌ Do NOT touch persona/Final styling logic (that moved to Phase V)
- ❌ SSE protocol changes (A.5) MUST be backward-compatible with existing
  bb-stream-* events the frontend already handles
- ❌ Failure is NOT a state — TURN_EXHAUSTED, WASH_FAILED, etc. are
  TERMINAL_REASONs on transitions, never enum members of State
- ❌ Identity threading (A.1) does NOT change message wire format unless
  necessary — TurnId can live as Pydantic field with default factory; old
  clients ignoring the field stay compatible
- ❌ NO StateContext / god-object carrying multi-step data — args/returns only
- ❌ NO explicit transition table / StateMachine class — coroutine flow IS the machine
- ❌ NO message queue inside the agent for step-to-step data — direct returns
- ❌ Phase label (self._phase) is for observation only — never drives control flow

## Hard Sealing Conditions

1. (Phase introspection) self._phase reflects the current async step at all
   times; queryable for SSE/logging. Phase enum has no failure variants.
2. (Pure refactor) Identical conversation produces byte-identical SSE stream
   before/after A.3. Step methods communicate only via args/returns — grep
   confirms no shared mutable StateContext.
4. (Await fix — the E1 root) "爬取论文" → REAL papers in final reply,
   verified by paper IDs cross-referenced with `audit_log.csv`.
5. (Lifecycle) After a long task completes and reply streams, next user
   message in the same session enters new ROUTING state cleanly with
   correct message history (no leftover state from previous turn).
6. (Event resume, not poll) AWAITING_TASK exits via TaskService event,
   not via polling loop (verified by absence of timed loop in the resume path).

## Design Decisions

- **Coroutine-native, not reified FSM**: async/await IS the state machine.
  Steps are async methods; suspension points are `await`s; phase is a thin
  label (self._phase) for introspection/SSE/logging, not a transition table.
  Justified by: single-process, near-linear flow, jsonl-recoverable sessions.

- **Narrow result objects, not a god context**: each step takes what it
  needs (args) and returns what it produces (a narrow result like
  RouteResult{probe_response, planned_calls, wash_context, probe_for_cmd}).
  No shared StateContext bag. wash_context flows as an explicit return field,
  not a cross-state closure local.

- **self.messages stays agent-level, not in any per-turn context**: messages
  are cross-turn accumulated state; per-turn data lives in step results.

- **Pure refactor before async feature (A.3 vs A.4)**: A.3 converts the
  while-loop to a coroutine flow with identical behavior. A.4 adds the
  await-suspension capability. Two separate HSCs isolate "refactor broke
  nothing" from "suspension works."

- **Suspension state is NOT serializable (accepted limitation)**: if Oligo
  crashes mid-await, the suspended turn is lost — user re-triggers. Acceptable
  for single-user local single-process. Persistent suspension = Phase VI+,
  requires explicit friction signal.

- **Failure is transition terminal, not state**: TerminalReason enum
  (COMPLETED / TURN_EXHAUSTED / CLIENT_GONE / LLM_TIMEOUT / TASK_FAILED) is
  returned by the turn coroutine; never a Phase label member.

- **Identity-first (A.1), then process schema (A.2)**: identities are the
  stable contract that state context objects bind to. `TurnId` must be
  threaded before state contexts reference it. DDD discipline: identities
  before aggregates.

- **State schema is user-defined, not auto-derived**: A.0 audit reports
  observed states and transitions but does NOT propose a canonical schema.
  The schema is an architectural decision the user makes after reading the
  audit. Claude implements; user designs.

- **Suspend/resume, NOT parallel turns**: "async" here means the agent can
  pause on AWAITING_TASK and resume on a `TaskService` event, freeing the
  loop while a long task runs. It does NOT mean concurrent turns.
  Concurrency is explicitly out of scope — single logical thread,
  event-driven suspension.

- **E1 root fix is await, not prompt**: E1 recurred twice (fabricated docs,
  then fabricated audit report). The root cause is the absence of an await
  state, not a weak persona prompt. A.4 installs AWAITING_TASK; the agent
  waits for real results before generating final output. Final-as-styling
  constraint moves to Phase V — it polishes E1 but does not fix it.

## Out of Scope (→ Phase V)

- Node ontology K/T/I/D + ARA alignment (Phase V.A)
- deep_research tool (Phase V — consumes III.E.A subagent + IV async core)
- Insight → Claude Code bridge (Phase V — consumes III.E.B + IV async core)
- vault_query (Phase V)
- Final-as-styling constraint (Phase V — polishes E1, doesn't fix it)
- Archive semantic layer / III.E HSC 2 debt (Phase V)
- risk_tier / workflow tools (Phase V)
- Concurrent/parallel turns (Phase VI+ if ever)
