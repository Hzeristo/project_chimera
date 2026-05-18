# Intent Recognition & Degraded Prompting (III.B.3 IR.0–5)

**Phase:** III.B.3 (`IR`) · aligns with roadmap row `III.B.3 Intent Recognition` (`62:62:project_chimera/docs/ROADMAP.md`) | **Status:** Active  
**Updated:** 2026-05-17 (against commit `48b2b2a`)

## Purpose

Shrink router system prompts safely when verbose tool manifests exceed budgets, constrain router prose so tool identifiers stay whitelist-faithful, wrap executed tool payloads in a deterministic `<tool_result>` envelope for downstream synthesis, steer the model via **optional** reflection hints when rendered outcomes fail, emit **named SSE** telemetry for live UI timers, and keep transport on **plaintext tool DSL** (`<tool_call>` / `<CMD:…>`) rather than vendor-native tool channels.

## Architecture

```text
ToolRegistry ──► _render_tool_list(max_chars?)
        │                     ▲ budgets from _build_router_system_prompt
Router system ≤ 4000 chars ──┘

Router probe ──► parse/exec unchanged (TOOL_PROTOCOL)

ExecutedToolResult ──► _classify_render_outcome + _format_one_tool_result_xml
        │
        └──► _render_tool_results_for_llm (reflection hints capped)

SSE bb-tool-* ──► llm_client.rs emit ──► ActiveToolTelemetry (100 ms tick UI)
```

## API / Schema

### Router length budget (`IR.1`)

- **Whole router system ceiling:** `_ROUTER_SYSTEM_PROMPT_MAX_CHARS = 4000` (`71:72:project_chimera/crucible_core/src/oligo/core/agent.py`).
- **Progressive `{tool_list}` caps** tried **in order** until `len(body)` fits: `budgets = [None, 3200, 2400, 1800, 1200, 800]` paired with `_prompt_context(tool_list_max_chars=tb)` (`484:511:project_chimera/crucible_core/src/oligo/core/agent.py`). `None` ⇒ **verbose** tool listing without tightening `max_chars` on `_render_tool_list` (`484:489:project_chimera/crucible_core/src/oligo/core/agent.py`).

### Tri-tier tool manifests (`verbose` · `compact` · `micro`)

Implemented in **`_render_tool_list`** (`144:181:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`):

| Tier | Chooses when… | Rendering notes |
|------|---------------|-----------------|
| **Verbose** | `max_chars is None` **or** concatenated verbose block `len(verbose) ≤ max_chars` (`156:158:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`) | Per-tool Markdown-ish block w/ schema lines + XML+CMD exemplars (`101:126:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`). |
| **Compact** | Verbose overflows **but** compact fits | Single line per tool, description truncated **`>160` ⇒ `157 + "…"`** (`134:136:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`). |
| **Micro** | Compact still overflows | `- name [long_running]` only (`139:141:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`). |

Hard tail chop appends `\n…[tool_list truncated]` when micro still exceeds `max_chars` (`174:181:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`).

Router instruction surface also carries **ROUTER\_INTRO**:

- **Zero-arg contract:** Literal `<args>{}</args>`, omission of `<args>`, **or** empty element → `{}` semantics on parse (`39:41:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`; parser default `"{}"` at `88:89:project_chimera/crucible_core/src/oligo/core/tool_protocol.py`).
- **No fabricated tool identifiers:** Explicit policy lines (`44:45:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`).

Live-model adherence to `<args>{{}}</args>` is an **explicit partial**: **IR.1.2** (`15:17:project_chimera/docs/ACCEPTED_PARTIALS.md`). Tier-loss trade (**IR.1.1**) (`11:13:project_chimera/docs/ACCEPTED_PARTIALS.md`; compression behavior `160:171:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`).

### `<tool_result>` LLM-visible wrapper (`IR.2`)

Renderer `_format_one_tool_result_xml`:

- **Success:** `<tool_result status="success" call_id="…">\n(inner)\n</tool_result>` (`271:278:project_chimera/crucible_core/src/oligo/core/agent.py`).
- **Failure:** `<tool_result status="failed" reason="{ENUM}" call_id="…">\n(inner)\n</tool_result>` (`274:279:project_chimera/crucible_core/src/oligo/core/agent.py`) where `ENUM ∈ {DENIED, TIMEOUT, TOOL_ERROR, ARGS_INVALID, EMPTY_RESULT}` from `_classify_render_outcome` (`126:252:project_chimera/crucible_core/src/oligo/core/agent.py`).

`inner` always records tool metadata + args JSON + verbatim result body (`261:267:project_chimera/crucible_core/src/oligo/core/agent.py`). **`reason` only applies when outer `status="failed"`** (`274:278:project_chimera/crucible_core/src/oligo/core/agent.py`).

### Reflection hints (`IR.2` continuation)

Assembly in `_render_tool_results_for_llm` (`1068:1085:project_chimera/crucible_core/src/oligo/core/agent.py`):

1. **`_REFLECTION_HINT_FAILURE`** iff **any** wrapper rendered as `failed` (`1059:1063`, `1069:1070`; string `133:136:project_chimera/crucible_core/src/oligo/core/agent.py`). This includes **DENIED**, timeouts, parser-classified empties (`215:249:project_chimera/crucible_core/src/oligo/core/agent.py`).
2. **`_REFLECTION_HINT_EMPTY`** only when **classification reason** was **`EMPTY_RESULT` _and_** `_user_text_suggests_expectation` on latest user plain text (**keyword / substring heuristic** `_USER_EXPECTATION_KEYWORDS`, `178:182:project_chimera/crucible_core/src/oligo/core/agent.py`) yields true (`1071:1072`; empty hint text `137:139`).
3. **Hint list clipped** with `hint_lines[:3]` (`1073:1073:project_chimera/crucible_core/src/oligo/core/agent.py`).
4. **All rendered rows succeed (`success`)** ⇒ both flags stay false ⇒ **no reflection block** appended (implicit from `1069` guard).

Trailing instruction urges synthesis / avoid extra tooling (`1078:1083`).

### Tool telemetry SSE (`IR.3`) & Astrocyte UI

Backend emits **`event: bb-tool-start`** payloads containing `call_id`, `tool_name`, `started_at_ms` (`732:743:project_chimera/crucible_core/src/oligo/core/agent.py`) per allowed plan ahead of `_run_one`; **`event: bb-tool-done`** echoes `call_id`, `status`, `elapsed_ms` (`799:807`, `841:849:project_chimera/crucible_core/src/oligo/core/agent.py`).

**DENIED-only batches** emit **zero** pair events because `_execute_tool_calls` returns immediately when `allowed_plans` empties (`866:874:project_chimera/crucible_core/src/oligo/core/agent.py`) → matches **IR.3.1** (`19:21:project_chimera/docs/ACCEPTED_PARTIALS.md`). **DEBT‑007** notes parse-phase denials similarly lack telemetry (`19:19:project_chimera/docs/TECHNICAL_DEBT.md`).

Rust bridge forwards named events verbatim (`307:323:project_chimera/astrocyte/src-tauri/src/llm_client.rs`).

**Frontend:** Roadmap shorthand **«ActiveToolStrip»** corresponds to **`ActiveToolTelemetry.svelte`** (import `6:6:project_chimera/astrocyte/src/routes/+page.svelte`, mount `<ActiveToolTelemetry />` `1868:1868:project_chimera/astrocyte/src/routes/+page.svelte`). Behaviour:

- `setInterval(pulseRunning, 100)` **10 Hz refresh** recomputes `displaySec` from **client `Date.now()` − `startedAtClientMs`** (`114:114:project_chimera/astrocyte/src/lib/ActiveToolTelemetry.svelte`, `40:53`, `73:87`).
- Elapsed renders `displaySec.toFixed(1)` with **`font-variant-numeric: tabular-nums`** (`137:138`, `183:183:project_chimera/astrocyte/src/lib/ActiveToolTelemetry.svelte`).
- Row removal uses **opacity 0.35s** plus **1000 ms defer** (`151:169:project_chimera/astrocyte/src/lib/ActiveToolTelemetry.svelte`; motion partial **IR.3.4** `23:25:project_chimera/docs/ACCEPTED_PARTIALS.md`).

### `xml_structured` preview slot

Composable XML dict injection pathway lives in **`PROMPT_MIDDLEWARE.md`** / `retrieval_context_demo`; production-scale PPR graph retrieval remains queued for **Phase IV** per roadmap (`106:113:project_chimera/docs/ROADMAP.md`) and flagged out of Phase III.C scope (`20:20:project_chimera/docs/phases/Phase-III.C.md`).

## Decision Points

- **Degrade manifests before truncating prose:** shrinking `{tool_list}` via tier steps preserves static router rules while staying below **4000** chars (`483:511:project_chimera/crucible_core/src/oligo/core/agent.py`).
- **Render-time failure taxonomy differs from runtime `ToolCallStatus`:** SUCCESS rows whose body resembles errors or empties downgrade to **`failed` + reason** (`243:249:project_chimera/crucible_core/src/oligo/core/agent.py`).
- **Hints are textual nudges, not replays:** they never issue new tool calls by themselves—the model must choose the next probe (`133:136`, `1078:1083:project_chimera/crucible_core/src/oligo/core/agent.py`).
- **Phase III.C red line forbids migrating to vendor-native function/tool APIs** (“No new dependency on OpenAI `response_format` / function calling APIs”, `97:98:project_chimera/docs/ROADMAP.md`)—intent stack stays DSL-in-text.

## Checklist: Extending Intent Behavior

1. When changing tiers, sync `_render_tool_list` thresholds & logging identifiers (`159:171:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`) **and** update router budget loops if totals shift (`483:508:project_chimera/crucible_core/src/oligo/core/agent.py`).
2. Update **ROUTER\_INTRO** caution text only if XML/CMD contract changes (`31:61:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`)—keep parser `tool_protocol` aligned.
3. New empty-detect phrases → broaden `_EMPTY_SUCCESS_MARKERS` / `_success_payload_behaves_like_tool_failure` consciously (`157:206:project_chimera/crucible_core/src/oligo/core/agent.py`).
4. Adjust reflection copy via `_REFLECTION_HINT_*`; remember **triple cap** already enforced (`133:139:project_chimera/crucible_core/src/oligo/core/agent.py`, `1073:1073:project_chimera/crucible_core/src/oligo/core/agent.py`).
5. New telemetry payload keys require matching updates in **`llm_client.rs` emit shapes** (`307:323:project_chimera/astrocyte/src-tauri/src/llm_client.rs`) **and** `ActiveToolTelemetry` guards (`73:112:project_chimera/astrocyte/src/lib/ActiveToolTelemetry.svelte`).
6. UI row timing/visual tweaks maintain **tabular numerals + tick cadence** expectations (`114:114`, `180:184:project_chimera/astrocyte/src/lib/ActiveToolTelemetry.svelte`; partial **IR.3.4** if animation budget changes).

## Known Issues / Accepted Partials

- **DEBT‑005:** Aggressive premature **compact/micro** under moderate tool counts—monitor after Phase III.C (`17:17:project_chimera/docs/TECHNICAL_DEBT.md`).
- **IR.1.1 / IR.1.2:** Tool-list compression & zero-arg live-LLM risk (`11:17:project_chimera/docs/ACCEPTED_PARTIALS.md`).
- **IR.3.4:** Tool-row fade timing acceptance (`23:25:project_chimera/docs/ACCEPTED_PARTIALS.md`; implementation `151:169:project_chimera/astrocyte/src/lib/ActiveToolTelemetry.svelte`).
- **`xml_structured` demo** inactive by default in router `active_ids`—see PROMPT Middleware doc checklist.

### Explicit Non-Goals

| Topic | Evidence / rationale |
|-------|-----------------------|
| **Provider-native assistant “tool” / JSON-schema calling** | Repo red line forbids attaching new OpenAI **`response_format` / function-calling stacks** (`97:98:project_chimera/docs/ROADMAP.md`). Router probes stay `generate_raw_text` over chat messages (`1109:1112:project_chimera/crucible_core/src/oligo/core/agent.py`). |
| **Automatic deterministic tool replays (“auto-retry”)** | Each planned tool runs once inside `_execute_tool_with_deadline` (`674:691:project_chimera/crucible_core/src/oligo/core/agent.py`); textual reflection only *advises* manual follow-up (`133:136`). |
| **DAG / speculative parallel dependency graphs beyond registry batches** | Concurrency emerges from **`partition_tool_calls` greedy batches**, not topological scheduling (`66:94:project_chimera/crucible_core/src/oligo/tools/registry.py`). |

## Cross-references

- [`PROMPT_MIDDLEWARE.md`](./PROMPT_MIDDLEWARE.md) — stable/dynamic assembly feeding router/final prompts.
- [`TOOL_PROTOCOL.md`](./TOOL_PROTOCOL.md) — DSL parsing, deny semantics feeding `<tool_result>`.
- [`SSE_PROTOCOL.md`](./SSE_PROTOCOL.md) — multiplexing telemetry vs stream chunks (`bb-sys-event`, etc.).
- **`DESIGN_SYSTEM.md`** — token usage for telemetry rows inherits Astrocyte variables (`143:186:project_chimera/astrocyte/src/lib/ActiveToolTelemetry.svelte`).
