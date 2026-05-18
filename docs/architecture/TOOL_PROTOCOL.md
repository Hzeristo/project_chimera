# Tool Protocol (`ToolSpec`, XML `<tool_call>`, `<CMD:…>`)

**Phase:** III.B.2 (TP.0–5) · III.B.3 (`<tool_result>` render, IR telemetry) | **Status:** Active  
**Updated:** 2026-05-17 (against commit `48b2b2a`)

## Purpose

Oligo models tool calls as text emitted by the router probe: canonical XML `<tool_call name="…">…</tool_call>` with an optional `<args>…</args>` payload, alongside a legacy `<CMD:tool(JSON)>` form that shares one regex (`CMD_ARG_REGEX`). Parsing, conservative JSON repairs, whitelist gating, batching by `ToolSpec.concurrency_safe`, execution, and LLM-facing `<tool_result>` wrappers are split across `tool_protocol.py`, `schemas.py`, and `agent.py`.

## Architecture

```text
Router probe text
  → strip fenced/inline Markdown for matching only (`agent._strip_markdown_code_for_cmd_extraction`)
  → `parse_tool_calls_unified` (XML + CMD, document order)
  → per-call `parse_args_with_repair` + registry / skill whitelist → `PlannedToolCall`
  → `partition_tool_calls` → asyncio gather per batch
  → `ExecutedToolResult` → `_render_tool_results_for_llm` (+ optional wash earlier)
```

## API / Schema

### `ToolSpec` (TP.1)

| Field | Semantics |
|-------|-----------|
| `name` | Registry key; must equal `TOOL_REGISTRY` / HTTP allowlist terminology (`578:579:project_chimera/crucible_core/src/crucible/core/schemas.py`). |
| `description` | One-line synopsis consumed in router-facing tool listings (`579:579:project_chimera/crucible_core/src/crucible/core/schemas.py`). |
| `args_schema` | JSON-schema-like dict for router prompt (`query`/`path`/`type`/`required`/`help`) (`580:582:project_chimera/crucible_core/src/crucible/core/schemas.py`). |
| `concurrency_safe` | `True`: may share a concurrent batch with other safe calls; `False`: exclusive batch (`584:587:project_chimera/crucible_core/src/crucible/core/schemas.py`, `61:63:project_chimera/crucible_core/src/oligo/tools/registry.py`). |
| `long_running` | `True` when implementation returns **`task_id` immediately** and user polls `check_task_status` (`588:591:project_chimera/crucible_core/src/crucible/core/schemas.py`; copy in miner specs `216:237:project_chimera/crucible_core/src/oligo/tools/registry.py`, `239:267:project_chimera/crucible_core/src/oligo/tools/registry.py`). Does **not** change `partition_tool_calls` grouping. |
| `examples` | Optional short strings “for router in-context learning” (`592:595:project_chimera/crucible_core/src/crucible/core/schemas.py`). **No default registrations set this field today** (`100:285:project_chimera/crucible_core/src/oligo/tools/registry.py`). |

Backward-compatible dict view: `TOOL_REGISTRY = { name: fn }` materialized lazily (`35:39:project_chimera/crucible_core/src/oligo/tools/__init__.py`).

### Parsing: XML `<tool_call>` vs legacy `<CMD:name(args)>`

| Concern | Location |
|---------|----------|
| XML matcher (attrs + body), `<args>` body regex, attribute fallback | `21:94:project_chimera/crucible_core/src/oligo/core/tool_protocol.py` |
| CMD matcher (shared with sanitizer / S0.4) | `9:18:project_chimera/crucible_core/src/oligo/core/text_sanitizer.py`; `TOOL_CALL_CMD_PATTERN = CMD_ARG_REGEX` (`15:18:project_chimera/crucible_core/src/oligo/core/tool_protocol.py`) |
| Unified merge sorted by appearance | `123:143:project_chimera/crucible_core/src/oligo/core/tool_protocol.py` |
| Missing `<args>` in XML ⇒ `"{}"` | `88:89:project_chimera/crucible_core/src/oligo/core/tool_protocol.py` |
| Probe-level Markdown strip before matching | `95:103:project_chimera/crucible_core/src/oligo/core/agent.py`; delegated to `TextSanitizer.strip_code_blocks_for_tool_matching` |

### Planned calls

`PlannedToolCall`: `id`, `tool_name`, `raw_args`, parsed `args`, `allowed`, `deny_reason`, `repairs_applied` (`405:434:project_chimera/crucible_core/src/crucible/core/schemas.py`). Produced in `_parse_tool_calls` (`542:630:project_chimera/crucible_core/src/oligo/core/agent.py`).

### Argument repair pipeline (TP.3)

`parse_args_with_repair`:

1. `strip()` empty string ⇒ `{}` (`234:237:project_chimera/crucible_core/src/oligo/core/tool_protocol.py`).
2. Direct `json.loads` on success ⇒ `_coerce_parsed_json_to_tool_args` (`238:240:project_chimera/crucible_core/src/oligo/core/tool_protocol.py`; coercion `213:224:project_chimera/crucible_core/src/oligo/core/tool_protocol.py`).
3. Else `attempt_argument_repair(raw_args)` applies **deterministic transforms in this order**, then `json.loads` (`244:251:project_chimera/crucible_core/src/oligo/core/tool_protocol.py`; `165:207:project_chimera/crucible_core/src/oligo/core/tool_protocol.py`).

| Order | Rule (`repairs_applied`) | Worked example | Code |
|-------|---------------------------|----------------|------|
| 1 | `strip_code_fence` | `` ```json\n{"q":"x"}\n``` `` → `{"q":"x"}` | `175:178:project_chimera/crucible_core/src/oligo/core/tool_protocol.py` |
| 2 | `single_to_double_quote` | `{'x': 1}` → `{"x": 1}` (only runs when `"` absent) | `180:182:project_chimera/crucible_core/src/oligo/core/tool_protocol.py` |
| 3 | `trailing_comma` | `{"a": 1 , }` trims comma before `}` | `184:187:project_chimera/crucible_core/src/oligo/core/tool_protocol.py` |
| 4 | `wrap_braces` | Fragment with `:` but no leading `{`/`[` wraps as `{ … }` | `189:195:project_chimera/crucible_core/src/oligo/core/tool_protocol.py` |
| 5 | `smart_quotes` | `“foo”` → `"foo"` (U+201C / U+201D) | `197:202:project_chimera/crucible_core/src/oligo/core/tool_protocol.py` |
Parsed JSON **must yield a dict** (strings coerced to `{"query": <str>}`); arrays alone fail coercion after repair (`204:211:project_chimera/crucible_core/src/oligo/core/tool_protocol.py`). Total failure ⇒ `ValueError` (`227:253:project_chimera/crucible_core/src/oligo/core/tool_protocol.py`), mapped by agent to denied plans (`572:595:project_chimera/crucible_core/src/oligo/core/agent.py`).

### `partition_tool_calls` (TP.5)

Keeps invocation order (`73:94:project_chimera/crucible_core/src/oligo/tools/registry.py`):

1. Accumulate contiguous `concurrency_safe=True` tools into one batch.
2. Each `False` closes the batch, then forms a singleton batch (`81:89`).
3. Flush remainder (`91:93`).

Registry lookup via `registry.is_concurrency_safe` (`61:63:project_chimera/crucible_core/src/oligo/tools/registry.py`).

Execution: single-tool batch `await`; multi-tool `asyncio.gather` (`794:837:project_chimera/crucible_core/src/oligo/core/agent.py`).

### Registered tools (defaults)

Registered in `_register_default_tools` (`100:285:project_chimera/crucible_core/src/oligo/tools/registry.py`). The **description** column paraphrases each `description=` argument in that block.
| `name` | `concurrency_safe` | `long_running` | One-line description |
|--------|--------------------|----------------|----------------------|
| `search_vault` | Yes | No | Keyword search in vault note bodies |
| `read_vault_file` | Yes | No | Read full note by path |
| `search_vault_attribute` | Yes | No | Frontmatter substring search |
| `obsidian_graph_query` | Yes | No | Obsidian graph BFS/query |
| `web_search` | Yes | No | DuckDuckGo search (no API key) |
| `arxiv_miner` | No | Yes | Fetch arXiv → Markdown; returns `task_id` |
| `daily_paper_pipeline` | No | Yes | Daily paper pipeline; returns `task_id` |
| `check_task_status` | Yes | No | Poll background task |

### Structured `<tool_result>` (III.B.3)

Render helper wraps each executed row (`255:279:project_chimera/crucible_core/src/oligo/core/agent.py`):

- Success: `<tool_result status="success" call_id="…">\n(inner)\n</tool_result>`
- Failure: `<tool_result status="failed" reason="…" call_id="…">\n(inner)\n</tool_result>`

`reason` derives from **`_classify_render_outcome`** (DENIED/TIMEOUT/ERROR heuristics, empty-body vs tool-error wording) (`215:252:project_chimera/crucible_core/src/oligo/core/agent.py`). Aggregation message adds optional reflection hints from overall batch (`1040:1085:project_chimera/crucible_core/src/oligo/core/agent.py`; hint strings `133:139:project_chimera/crucible_core/src/oligo/core/agent.py`).

### SSE hooks (telemetry, not persistence)

Named events emitted via `sse_event` (`8:10:project_chimera/crucible_core/src/oligo/core/sse.py`): per-plan `bb-tool-start` payload (`736:742`), completion `bb-tool-done` (`799:807`, `842:849`), optional batch prelude `phase=batch_start` through `_sse_data(__SYS_TOOL_CALL__…)` (`884:891`). **DENIED‑only parses** yield no allowed plans → `_execute_tool_calls` returns early with **zero** telemetry frames (`866:874:project_chimera/crucible_core/src/oligo/core/agent.py`); matches **IR.3.1** (`19:21:project_chimera/docs/ACCEPTED_PARTIALS.md`). See also **`INTENT_AND_DEGRADATION.md`** / **`SSE_PROTOCOL.md`**.

Tri-tier verbosity for **listed** router tools (`verbose`/`compact`/`micro`) plus hard truncate lives in **`PROMPT_MIDDLEWARE.md`** / `_render_tool_list` (`144:181:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`), not inside `registry.py`.

## Decision Points

- **Dual syntax surface** persists for migration; unified ordering prevents silent CMD/XML shadowing (`123:143:project_chimera/crucible_core/src/oligo/core/tool_protocol.py`).
- **Markdown literals first**: strip fenced/inline spans before extraction so explanatory examples remain inert (`95:103:project_chimera/crucible_core/src/oligo/core/agent.py`).
- **Repair is format-only**: no speculative semantic fixes; unrecognized JSON after pipeline stays denied (`572:594:project_chimera/crucible_core/src/oligo/core/agent.py`).
- **Concurrency batches follow registry flags**, independent of runtime duration or `long_running` (`66:94:project_chimera/crucible_core/src/oligo/tools/registry.py`).

## Checklist: Adding a Tool

1. Implement callable returning `Awaitable[str]` (or **`task_id` string** pattern for polls) beside existing modules (`miner_tools.py`, `vault_tools.py`, …).
2. Register with **`ToolRegistry.register(..., ToolSpec(...))`** inside `_register_default_tools`; choose **`concurrency_safe`** / **`long_running`** consciously (`215:267:project_chimera/crucible_core/src/oligo/tools/registry.py` illustrates both patterns).
3. Fill **`args_schema`** (`type`/`required`/`help`) — router templating derives examples when empty optional sets exist (`101:126:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`).
4. Optionally populate **`examples`** if default prompts need illustrative strings (`592:595:project_chimera/crucible_core/src/crucible/core/schemas.py`).
5. Wire tests in `tests/oligo/` (parse/execution/registry) mirroring **`test_tool_protocol.py`** conventions.
6. If policy-sensitive, validate skill allowlists / metrics expectations in **`ChimeraAgent`** integration paths (`602:627:project_chimera/crucible_core/src/oligo/core/agent.py`).

## Known Issues

- **DEBT-004**: `_execute_tool` no longer needs to re-parse `raw_args` once `PlannedToolCall.args` exists, yet legacy scaffolding remains (`16:16:project_chimera/docs/TECHNICAL_DEBT.md`; dispatch `646:671:project_chimera/crucible_core/src/oligo/core/agent.py`).
- **DEBT-007**: Tool telemetry skips **parse-stage** rejects (malformed XML before `PlannedToolCall` rows materialize); only execution-phase paths emit SSE (`19:19:project_chimera/docs/TECHNICAL_DEBT.md`; compare `_parse_tool_calls` `556:594:project_chimera/crucible_core/src/oligo/core/agent.py` vs `_execute_tool_calls` `866:896:project_chimera/crucible_core/src/oligo/core/agent.py`).
- **IR.3.1** / **DENIED‑only batches**: documented accepted behavior — no **`bb-tool-start` / `bb-tool-done`** when nothing executes (`19:21:project_chimera/docs/ACCEPTED_PARTIALS.md`; `866:874:project_chimera/crucible_core/src/oligo/core/agent.py`).

## Cross-references

- [`PROMPT_MIDDLEWARE.md`](./PROMPT_MIDDLEWARE.md) — how `tool_list` is rendered into router system text.
- [`INTENT_AND_DEGRADATION.md`](./INTENT_AND_DEGRADATION.md) — IR tier budgets, hints, telemetry semantics beside tool rows.
- [`SSE_PROTOCOL.md`](./SSE_PROTOCOL.md) — `sse_event`, `bb-tool-*`, multiplexing with streaming chunks.
