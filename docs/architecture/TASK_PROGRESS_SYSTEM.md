# Task Progress System (`TaskService` → SSE → Tauri → `ActiveTaskPanel`)

**Phase:** III.A Step 2 (Event-Driven Task Progress) (`48:48:project_chimera/docs/ROADMAP.md`) | **Status:** Active  
**Updated:** 2026-05-17 (against commit `48b2b2a`)

## Purpose

Background jobs (miner pipelines, polling surfaces) mutate JSON task records on disk and push structured **`TaskEvent`** rows to SSE subscribers. Astrocyte’s Tauri layer maintains a resilient EventSource subscription, renames payloads for the SPA, and **`ActiveTaskPanel`** renders coarse progress bars and **local-clock** dwell timers **without tying elapsed display to backend `timestamp_ms`**.

## Architecture

| Layer | Role |
|-------|------|
| **1 · Python `TaskService`** | Persists `{task_id}.json`, broadcasts `emit_event()` into an internal **`_event_queue`** plus **`_subscribers`** fan-out queues (`59:109:project_chimera/crucible_core/src/crucible/services/task_service.py`). |
| **2 · FastAPI `/v1/tasks/stream`** | Opens one subscriber queue per HTTP client, yields SSE (`task-*` events + keep-alives; unsubscribe on teardown) (`183:218:project_chimera/crucible_core/src/oligo/api/server.py`). App wires default `TaskService` at startup (`54:56:project_chimera/crucible_core/src/oligo/api/server.py`). |
| **3 · Rust `task_stream.rs` + Tauri launcher** | Permanent EventSource subscription; filters infra noise; emits **`bb-task-event`** wrappers (`10:76:project_chimera/astrocyte/src-tauri/src/task_stream.rs`). Spawned via `spawn(async { run_task_stream_loop(...).await })` inside **`setup`** (`1399:1401:project_chimera/astrocyte/src-tauri/src/lib.rs`) — orthogonal to ephemeral `/v1/agent/invoke` streams. |
| **4 · Svelte `ActiveTaskPanel`** | Listens **`bb-task-event`**, merges rows; **100 ms ticker** recomputes per-stage dwell from **`Date.now()` − `stage_started_at_ms`** (`219:226:project_chimera/astrocyte/src/lib/ActiveTaskPanel.svelte`; `90:107`). |

## API / Schema

### `TaskEventType` enumeration

Canonical names feed SSE `event:` lines via `task-{enum.value}` (`18:24:project_chimera/crucible_core/src/crucible/core/schemas.py`; emission `196:199:project_chimera/crucible_core/src/oligo/api/server.py`):

| Enum member | Serialized `event_type.value` | Example SSE event name |
|-------------|-------------------------------|-------------------------|
| `CREATED` | `"created"` | `task-created` |
| `STAGE_START` | `"stage_start"` | `task-stage_start` |
| `STAGE_PROGRESS` | `"stage_progress"` | `task-stage_progress` |
| `COMPLETED` | `"completed"` | `task-completed` |
| `FAILED` | `"failed"` | `task-failed` |
| `CANCELLED` | `"cancelled"` | `task-cancelled` |

Rust filters only **`task-*`** (after dropping heartbeats/hello) (`38:43:project_chimera/astrocyte/src-tauri/src/task_stream.rs`). **Envelope** forwarded to frontend:

```json
{ "event_type": "<SSE event name>", "payload": <TaskEvent JSON> }
```

(`47:52:project_chimera/astrocyte/src-tauri/src/task_stream.rs`; listener `220:222:project_chimera/astrocyte/src/lib/ActiveTaskPanel.svelte`)

### `TaskEvent` payload

| Field | Notes |
|-------|-------|
| `event_type` | Transport enum (also duplicated as SSE outer name). |
| `task_id` / `task_type` | Correlates filesystem row (`600:607:project_chimera/crucible_core/src/crucible/core/schemas.py`). |
| `stage_id` | **Mandatory on canonical `start_stage`** (`135:137:project_chimera/crucible_core/src/crucible/services/task_service.py`; schema semantics `608:614:project_chimera/crucible_core/src/crucible/core/schemas.py`). Nullable on other emits. |
| `stage_label` | Human stage copy (`612:614:project_chimera/crucible_core/src/crucible/core/schemas.py`; UI prefers this over IDs `257:259:project_chimera/astrocyte/src/lib/ActiveTaskPanel.svelte`). |
| `overall_progress` | Clamp `0…1` in service helpers (`289:294:project_chimera/crucible_core/src/crucible/services/task_service.py`; note **A1.1** approximation caveat below). |
| `message` / `error` | Created/completed chatter vs failures (`619:622:project_chimera/crucible_core/src/crucible/core/schemas.py`). |
| `timestamp_ms` | Wall clock snapshot at emit (`622:622:project_chimera/crucible_core/src/crucible/core/schemas.py`; **elapsed UI ignores it intentionally** — see Decision Points). |

### SSE heartbeat / hello framing

Handshake + idle keep-alive are **purpose-built control events**, not surfaced as `bb-task-event`:

- Initial `task-stream-hello` plus `subscriber.get()` timeouts emit `task-heartbeat` frames (`191:203:project_chimera/crucible_core/src/oligo/api/server.py`).
- Rust **`continue`**s without emitting when encountering them (`39:43:project_chimera/astrocyte/src-tauri/src/task_stream.rs`).  
  ⇒ **Heartbeat / handshake filter list (non-UI events): `task-heartbeat`, `task-stream-hello`.**

Disconnect handling uses shared `CLIENT_GONE_EXCEPTIONS` guard (`205:207:project_chimera/crucible_core/src/oligo/api/server.py`).

### Subscriber backpressure semantics

Internal buffer `Queue(maxsize=1000)`; `put_nowait` drops newest event on exhaustion (`93:103:project_chimera/crucible_core/src/crucible/services/task_service.py`). Per-subscriber FIFO size **100**, same drop policy (`105:103`).

### Reconnection backoff (Rust)

`RECONNECT_BACKOFF_MS = [500, 1000, 2000, 5000, 10000]` (`8:8:project_chimera/astrocyte/src-tauri/src/task_stream.rs`). Indices advance on connection build failure (`22:27`), stream errors (`65:74`), respecting `.min(len-1)` cap.

### `ActiveTaskPanel` stage timer semantics

Critical branch for **`task-stage_start`** (`136:155`):

- Compute `nextStageId` vs `previousStageId`.  
- If IDs differ (**or row missing**): reset `stage_started_at_ms = Date.now()`, zeroing dwell (`141:149`).  
- If only **label**/progress/metadata changes (`stage-progress` handler `159:174`): **`stage_started_at_ms` persists** (`162:169`).

Timer loop `refreshElapsedInStage` derives **`elapsed_in_stage_s = max(0, (Date.now() - stage_started_at_ms)/1000)`** (`90:100`); tick cadence `TICK_MS = 100` (`35:36`, `224:226`). Display `{elapsed.toFixed(1)}s` (`260`).

## Decision Points

- **Local clock for dwell:** Roadmap mandates **elapsed from `Date.now()`**, decoupled from jittery network timestamps (`48:48:project_chimera/docs/ROADMAP.md`); implemented via `refreshElapsedInStage` / `stage_started_at_ms`.
- **`stage_id` vs `stage_label`:** Stable machine key drives timer resets; textual label may update independently on `stage_progress`.
- **`timestamp_ms` carriage:** Serialized for potential skew diagnostics but **unused** by the panel timer path (no imports of payload timestamp in dwell math).
- **Filtered SSE noise:** Only domain `task-*` crosses into UI layer; infra heartbeats avoided.
- **`tabular-nums`:** Roadmap cites **tabular-numeric typography** beside the timer (`48:48:project_chimera/docs/ROADMAP.md`), yet **`ActiveTaskPanel.svelte` carries no scoped `font-variant-numeric`**. **TBD — verify whether global Astrocyte CSS supplies tabular digits or typography debt remains.**

## Checklist: Onboarding a New Task Kind

1. Choose `task.type` identifier when **`TaskService.create_task(...)`** is invoked (`257:276:project_chimera/crucible_core/src/crucible/services/task_service.py`).
2. Emit lifecycle rows using **`emit_created`**, **`start_stage` / `emit_stage_progress`**, terminal **`emit_completed|failed|cancelled`** as appropriate (`114:251:project_chimera/crucible_core/src/crucible/services/task_service.py`).
3. Guarantee **stable `stage_id` strings per logical phase** if UI must reset timers; keep **human copy in `stage_label`**.
4. Clamp progress via **`_safe_progress`** expectations (respect **A1.1** caveat for coarse estimates (`47:49:project_chimera/docs/ACCEPTED_PARTIALS.md`).
5. Add / extend miner integration tests exercising subscriber fan-out (`tests/oligo/test_task_event_bus.py` patterns).
6. Verify Astrocyte still filters unknown events cleanly (Rust `task-` guard) (`38:63:project_chimera/astrocyte/src-tauri/src/task_stream.rs`).

## Known Issues / References

- **A1.1 — `overall_progress` linear guesses** for long pipelines (`47:49:project_chimera/docs/ACCEPTED_PARTIALS.md`): UI percentages are indicative, not wall-clock proportional.
- **Telemetry drops** when subscriber queues overflow (`93:103`, `99:103:project_chimera/crucible_core/src/crucible/services/task_service.py`).
- **DEBT‑006** (stage-card persistence UX) loosely adjacent to archival expectations (`18:18:project_chimera/docs/TECHNICAL_DEBT.md`).
- Typography gap (**tabular-nums**) noted under Decision Points.

## Cross-references

- [`SSE_PROTOCOL.md`](./SSE_PROTOCOL.md) — `sse_event(...)` framing shared with invoke stream channels (`27:27:project_chimera/crucible_core/src/oligo/api/server.py`).
- [`CONFIG_SCHEMA.md`](./CONFIG_SCHEMA.md) — `~/.chimera` roots via `get_chimera_root()` (`25:25:project_chimera/crucible_core/src/crucible/core/platform.py`); runtime task dir `get_chimera_root() / "tasks"` (`54:55:project_chimera/crucible_core/src/oligo/api/server.py`).
- [`INTENT_AND_DEGRADATION.md`](./INTENT_AND_DEGRADATION.md) — distinct telemetry channel vs `bb-tool-*` tool-strip events.
