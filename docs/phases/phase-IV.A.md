# Phase IV.A — Async Agent Core

**Status:** Planned (start after III.E/III.F sealed + hybrid week friction)
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

Therefore Phase IV.A builds, in order:
  Layer 2 — Identities (TurnId, explicit Conversation/Turn contexts)
  Layer 3 — Process DDD (State enum, StateContext, StateTransition)
  FSM refactor (replace while-loop, behavior-preserving)
  THEN async (AWAITING_TASK as a state, suspend/resume on TaskService events)

Async is a consequence of Layer 3, not a parallel goal.

## Sprint Sequence

| Sprint | One-line goal | Status |
|---|---|---|
| A.0 | Audit current theater loop, identify implicit states, blocking awaits, data flows, AWAITING_TASK gap | Sealed |
| A.1 | Identity DDD (Layer 2): TurnId + explicit Conversation/Turn context objects; threaded through ChatMessage, ExecutedToolResult, etc. Schema-only, no executor change | Pending |
| A.2 | Process DDD (Layer 3): State enum, StateContext, StateTransition objects, StateMachine class. Schema + scaffolding only, not yet wired to agent.py | Pending |
| A.3 | FSM executor refactor: replace `_run_theater_stream` while-loop with StateMachine-driven flow; cross-state locals (`wash_context`, `probe_for_cmd`) move into StateContext. PURE REFACTOR — byte-identical observable behavior | Pending |
| A.4 | AWAITING_TASK state: long_running tools suspend; resume via TaskService event subscription; real result re-enters the FSM | Pending |
| A.5 | SSE protocol upgrade: state-transition events streamed mid-turn (frontend learns when agent is awaiting) | Pending |
| A.6 | Lifecycle integrity: agent processes next user message correctly after a long-task await completes | Pending |

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

## Hard Sealing Conditions

1. (Identity) Every `ChatMessage`, `ExecutedToolResult`, `TaskEvent`, and SSE
   frame in a turn carries the same `TurnId`; `TurnId` derivable from any
   single artifact in the turn.
2. (Process schema) `State` is an enum without failure variants; every
   `StateTransition` has either `next_state` or `terminal_reason`, never both.
3. (Pure refactor — A.3) Identical conversation produces identical SSE byte
   stream before/after A.3 (regression suite).
4. (Await fix — the E1 root) "爬取论文" → REAL papers in final reply,
   verified by paper IDs cross-referenced with `audit_log.csv`.
5. (Lifecycle) After a long task completes and reply streams, next user
   message in the same session enters new ROUTING state cleanly with
   correct message history (no leftover state from previous turn).
6. (Event resume, not poll) AWAITING_TASK exits via TaskService event,
   not via polling loop (verified by absence of timed loop in the resume path).

## Design Decisions

- **Hand-rolled FSM, asyncio primitives only (no framework)**: consistent
  with Chimera's anti-framework philosophy. The user has built async
  pipelines before (graduation project: four stateless daemons consuming
  global frame DTOs, UUID aggregation, poison-pill boundary broadcast).
  This is hand-rollable in ~2 days with Claude Code long-running + oversight.

- **Three-layer DDD, not two**: Layer 1 (data) is done — `PlannedToolCall`,
  `ExecutedToolResult`, `ChatMessage` are typed. Layer 2 (identities:
  `TurnId`, explicit Conversation/Turn context) and Layer 3 (process: `State`
  enum, `StateContext`, `StateTransition`, `StateMachine`) are Phase IV.A's
  deliverable. Async is a Layer 3 consequence, not a parallel objective.

- **Identity-first (A.1), then process schema (A.2)**: identities are the
  stable contract that state context objects bind to. `TurnId` must be
  threaded before state contexts reference it. DDD discipline: identities
  before aggregates.

- **Failure is transition, not state**: `TURN_EXHAUSTED` in the audit is a
  control-flow terminator, not a state. The `State` enum stays orthogonal
  to failure modes. Each `StateTransition` carries either `next_state` or
  `terminal_reason`, never both. Failure modes are enumerated as
  `TerminalReason` values, not `State` variants.

- **PRE_TURN_SETUP and PROBE_DRAFT_BACKFILL are real states**: the audit
  found them as "micro-states" but they have true invariants — system slot
  replacement, and the draft-vs-pass decision respectively. Squashing them
  into ROUTING hides decisions that belong in named `StateContext` variants.

- **wash_context and probe_for_cmd move into StateContext**: cross-state
  closure locals are an FSM anti-pattern. `wash_context` (captured at
  ROUTING exit, consumed in WASHING) and `probe_for_cmd` (captured during
  EXECUTING, consumed in RENDERING_TOOL_RESULTS) become typed fields on
  the appropriate `StateContext` variant. No implicit closure threading.

- **Pure refactor before feature (A.3 vs A.4)**: A.3 converts the sync
  while-loop to an FSM with byte-identical observable behavior. Only A.4+
  adds AWAITING_TASK / event capabilities. This isolates "did the refactor
  break anything" from "does the new feature work."

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
