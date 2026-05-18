# Server-Sent Events (SSE) Protocol

**Phase:** III.A Connection Convergence (invoke + task SSE) · III.B.3 tool named events (`bb-tool-*`) · III.C **`bb-message-artifacts` queued** (`67:71:project_chimera/docs/phases/Phase-III.C.md`) | **Status:** Active  
**Updated:** 2026-05-17 (against commit `48b2b2a`)

## Purpose

Oligo multiplexes Agent theater output, deterministic telemetry prefixes, optional tool-timing frames, and error beacons onto **one chunked HTTP SSE response** consumed by Astrocyte. A separate **long-lived** GET stream broadcasts task lifecycle envelopes. Unified helpers keep wire format consistent; Rust re-emits orthogonal Tauri events for Svelte subscribers.

## Architecture

```text
POST /v1/agent/invoke
  ├► unnamed `data: {"content":…}` frames (`_sse_data`) — telemetry + legacy payloads
  ├► `event: bb-stream-chunk` — final lexical tokens (`_sse_chunk`)
  ├► `event: bb-tool-*` — optional concurrency telemetry
  └► `event: bb-stream-done` — typed terminal beacons OR stream EOF (Rust adds synthetic DONE)

GET /v1/tasks/stream
  ├► `task-stream-hello`, `task-heartbeat` — keep-alives / handshake
  └► `task-{TaskEventType.value}` — background job fan-out (`task-stage_start`, …)

Rust llm_client: split routing + normalization → `emit("bb-sys-event" | "bb-stream-chunk" | …)`
Rust task_stream: drop heartbeats/hello → `emit("bb-task-event", envelope)`
```

## API / Schema

### `sse_event(event_type, data)`

Constructor for **named SSE** messages (`event:` + JSON `data:` + blank separator) (`8:10:project_chimera/crucible_core/src/oligo/core/sse.py`).

Contrast **`_sse_data(payload)`**, which emits the **legacy default event** shaped as **`data: {"content": …}\n\n`** without an `event:` line (`312:326:project_chimera/crucible_core/src/oligo/core/agent.py`). Router/tool/wash breadcrumbs use **`_sse_data("__SYS_TOOL_CALL__" + json)`** packing (`296:298:project_chimera/crucible_core/src/oligo/core/agent.py`, yields e.g. `1133:1162`, `1199:1210`, `1236:1237`, `1336:1342`).

### Event taxonomy

| Logical name | When emitted (server) | Wire shape / notes |
|--------------|-----------------------|---------------------|
| **(implicit / unnamed)** `data:` | Router + tool + wash + final “Generating…” breadcrumbs via **`_sse_data`** | JSON **`{"content": "__SYS_TOOL_CALL__{…}"}`** from **`_sys_telemetry_obj`** (`296:298:project_chimera/crucible_core/src/oligo/core/agent.py`) wrapped via **`_sse_data`** (`312:326:project_chimera/crucible_core/src/oligo/core/agent.py`) |
| **`bb-stream-chunk`** | Final streaming slice loop (`1345:1353:project_chimera/crucible_core/src/oligo/core/agent.py` via **`_sse_chunk` → `sse_event`** `328:330`) | `{"content": "<few chars>"}` |
| **`bb-stream-done`** | Router/final timeouts (`1119:1122`, `1324:1327`), exhausted turns fatal (`1362:1363`), client disconnect wrappers (`1375:1377`, `1380:1382`), HTTP wrapper errors (`148:153:project_chimera/crucible_core/src/oligo/api/server.py`) | **Error-ish:** `{ "error": true, "message": "..." }`; **abort:** `{ "aborted": true, "reason": "client_gone" }` (`1377`, `151:152` server mirrors `145`) |
| **`bb-tool-start` / `bb-tool-done`** | Tool runner optional queue (`884:891:project_chimera/crucible_core/src/oligo/core/agent.py` feeding frames `732:743`, `799:807`, `839:849`) | JSON payloads (call id, timings) — taxonomy cross-ref **`TOOL_PROTOCOL.md`** |
| **`bb-sys-event`** | **Rust-only** normalization of **`__SYS_TOOL_CALL__`** bodies into structured JSON (`332:356:project_chimera/astrocyte/src-tauri/src/llm_client.rs`); user-cancel stub string (`368:370:project_chimera/astrocyte/src-tauri/src/llm_client.rs`, `215:216` direct API path). | App-handle event name decouples UI from SSE default-event parsing. |
| **`bb-message-artifacts`** | Planned before `bb-stream-done` for structured attachments (**FC.2**) (`69:71:project_chimera/docs/phases/Phase-III.C.md`) | **Not implemented in codebase at doc time — verify when FC.2 lands.** |

#### Task-plane events (`GET /v1/tasks/stream`)

| Event | Role |
|-------|------|
| `task-stream-hello` | Opening handshake carrying `timestamp_ms` (`191:191:project_chimera/crucible_core/src/oligo/api/server.py`). |
| `task-heartbeat` | Idle watchdog every **`asyncio.wait_for(..., timeout=15.0)`** miss (`195:203:project_chimera/crucible_core/src/oligo/api/server.py`). |
| `task-<enum>` | e.g. `task-created`, `task-stage_start` via `sse_event(f"task-{event.event_type.value}", model_dump(...))` (`196:199:project_chimera/crucible_core/src/oligo/api/server.py`). Rust filters **`task-heartbeat`/`task-stream-hello`** then forwards only names beginning `task-` (`38:63:project_chimera/astrocyte/src-tauri/src/task_stream.rs`). Details: **`TASK_PROGRESS_SYSTEM.md`**. |

### Invoke channel vs persistent task channel

| Channel | Verb / path | Lifetime | Consumers |
|---------|-------------|----------|-----------|
| **Per-request theater** | `POST /v1/agent/invoke` (`98:99:project_chimera/crucible_core/src/oligo/api/server.py`) | Tied to one Agent session; streaming body returned via `StreamingResponse` (`173:176`) | **`llm_client::stream_oligo_agent`** (`231:376:project_chimera/astrocyte/src-tauri/src/llm_client.rs`) |
| **Task fan-out** | `GET /v1/tasks/stream` (`183:218:project_chimera/crucible_core/src/oligo/api/server.py`) | Indefinite `while True` subscriber loop (`192:207`) until disconnect | **`task_stream::run_task_stream_loop`** |

### Rust forwarding & heartbeat filtering

**Agent invoke parsing (`stream_oligo_agent`):**

- Named **`bb-stream-done`:** parse JSON fallback to synthetic error wrapper if malformed (`294:305:project_chimera/astrocyte/src-tauri/src/llm_client.rs`).
- **`bb-tool-start`/`bb-tool-done`:** verbatim JSON payloads → same-name Tauri events (`307:323`).
- Unstructured / JSON `content` lanes split between **`bb-sys-event`** normalization vs **`bb-stream-chunk`** append (`332:357`).
- Idle cancel emits **`bb-sys-event`** string sentinel (`368:370:project_chimera/astrocyte/src-tauri/src/llm_client.rs`).

**Important:** **`stream_oligo_agent` does not ingest `task-heartbeat`** — hearts exist solely on **`/tasks/stream`**, stripped in **`task_stream.rs`** before UI (`39:43:project_chimera/astrocyte/src-tauri/src/task_stream.rs`).

### Synthetic completion (`DONE`)

Successful theater completion **often ends without Oligo sending `event: bb-stream-done`**—the generator returns after emitting chunks (`1355:1356:project_chimera/crucible_core/src/oligo/core/agent.py`). Rust then finishes the HTTP SSE iterator, observes `Ok(Some(full_text))`, persists chat, and emits terminal **`emit("bb-stream-done", "DONE")`** (`978:979:project_chimera/astrocyte/src-tauri/src/lib.rs`). Conversely, **`Ok(None)`** means the Rust layer already forwarded an explicit **`bb-stream-done`** from Oligo and **must not** duplicate completion (`914:924:project_chimera/astrocyte/src-tauri/src/lib.rs`, `229:230:project_chimera/astrocyte/src-tauri/src/llm_client.rs`).

### Client-disconnect & transport errors (`CLIENT_GONE_EXCEPTIONS`)

- **Construction:** aggregated Starlette/socket disconnect types tuple (`69:69:project_chimera/crucible_core/src/oligo/core/agent.py`).
- **Inner generators (`_execute_tool_plan_batch`, parser loop):** propagate raw (`753:755`, `819:821:project_chimera/crucible_core/src/oligo/core/agent.py`).
- **Agent façade `run_theater`:** centralizes user-visible abort beacon (`1372:1383:project_chimera/crucible_core/src/oligo/core/agent.py`; docstring `1369:1370`).
- **FastAPI `safe_theater_stream` duplicate guard:** emits matching abort frame if middleware catches disconnect before agent wrapper (`136:146:project_chimera/crucible_core/src/oligo/api/server.py`).
- **Task SSE:** swallowed `CLIENT_GONE_EXCEPTIONS` unsubscribes quietly (`205:208:project_chimera/crucible_core/src/oligo/api/server.py`).
- **Pipe heuristic:** `_looks_like_pipe_broken` maps busted transports to **same aborted payload** (`1380:1382:project_chimera/crucible_core/src/oligo/core/agent.py`; heuristic `74:84:project_chimera/crucible_core/src/oligo/core/agent.py`).

Generic handler wrapper also maps unknown exceptions → `{error:true,...}` SSE tail (`147:153:project_chimera/crucible_core/src/oligo/api/server.py`).

## Decision Points

- **Named chunks vs unnamed telemetry:** Keeps lexical streaming (`bb-stream-chunk`) decoupled from router diagnostics still riding default-event frames interpreted by **`__SYS_TOOL_CALL__` sentinel** (`328:330:project_chimera/crucible_core/src/oligo/core/agent.py`, `_sys_telemetry_obj` `296:298:project_chimera/crucible_core/src/oligo/core/agent.py`).
- **Abort vs error payloads:** distinguishes transport/user abort (`aborted`) from LLM/gateway faults (`error`) (`1375:1383`, `1119:1122`).
- **Task-plane noise suppression:** Hearts never hit **`bb-task-event`** frontend listener (`39:43:project_chimera/astrocyte/src-tauri/src/task_stream.rs`).
- **Single completion authority:** avoids double DONE when streaming server already signaled terminal JSON (`914:924:project_chimera/astrocyte/src-tauri/src/lib.rs`).

## Checklist: Adding a New SSE Event

1. Define Python emission via **`sse_event`** (named) **or `_sse_data`** (legacy unnamed) consistently (`8:10:project_chimera/crucible_core/src/oligo/core/sse.py`; `312:326:project_chimera/crucible_core/src/oligo/core/agent.py`).
2. Extend **`stream_oligo_agent`** branching if desktops must parse distinct `msg.event` values (`292:357:project_chimera/astrocyte/src-tauri/src/llm_client.rs`).
3. Update Svelte listeners (`listen('…')`) and TypeScript payloads mirroring serde JSON shapes.
4. For background workers, reuse **`sse_event`** in FastAPI generators (pattern `183:218:project_chimera/crucible_core/src/oligo/api/server.py`).
5. Document partial acceptance if UI or smoke tests omit the event (append **`ACCEPTED_PARTIALS.md`**).
6. Re-run scripted smoke (**`scripts/smoke_intent_recognition.py`** for tool telemetry regression) (`89:99:project_chimera/crucible_core/scripts/smoke_intent_recognition.py`).

## Known Issues

- **`bb-message-artifacts` absent** despite Phase III.C plan — track FC.2 completion (`69:71:project_chimera/docs/phases/Phase-III.C.md`).
- **Success DONE type heterogeneity:** Oligo may omit server `bb-stream-done`, emitting plain string **`"DONE"`** from Rust only (`979:979:project_chimera/astrocyte/src-tauri/src/lib.rs`) versus JSON payloads from explicit server errors — frontend must tolerate both shapes.
- **Duplicate abort handling risk** (`run_theater` vs `safe_theater_stream`) mitigated via identical payload schemas but can still double-fire under racey disconnects (**document only**; cite `1375`, `143` pairs).

## Cross-references

- [`TOOL_PROTOCOL.md`](./TOOL_PROTOCOL.md) — `bb-tool-*` semantics + DENIED omission.
- [`INTENT_AND_DEGRADATION.md`](./INTENT_AND_DEGRADATION.md) — reflection + telemetry interplay.
- [`TASK_PROGRESS_SYSTEM.md`](./TASK_PROGRESS_SYSTEM.md) — `task-*` envelope schema + reconnect policy.
- [`CONFIG_SCHEMA.md`](./CONFIG_SCHEMA.md) — binds hosting `effective_oligo_base_url`.
