# Observability Logging (Phase I M3 baseline)

**Phase:** I M3 Observability Baseline (`19:19:project_chimera/docs/ROADMAP.md`) | **Status:** Active  
**Updated:** 2026-05-17 (against commit `48b2b2a`)

## Purpose

Chimera runs as **multiple processes** (Python Oligo + optional CLI scripts + Tauri/`Astrocyte`). Structured **shared column order** (`time | level | logger name / target | message`) and **stable bracket prefixes inside the message** make multi-window greps and correlation with SSE/task events workable without a SaaS observability stack. This page is the **cross-repo contract** for that baseline; narrative copies also exist under `crucible_core/docs/LOGGING_STANDARD.md` — treat **paths here** as authoritative toward `chimera-code-taste`.

## Architecture

```text
Python entrypoints (scripts / start_oligo):
  logging.basicConfig(...) at import time (`15:19:project_chimera/crucible_core/scripts/start_oligo.py`)
    → uvicorn inherits root formatter when launched via same process (`26:33:project_chimera/crucible_core/scripts/start_oligo.py`)

Astrocyte (Tauri) native:
  env_logger Builder + chrono-local timestamp mirrors Python pipes (`10:21:project_chimera/astrocyte/src-tauri/src/main.rs`)
  RUST_LOG default "debug" if unset (`7:9:project_chimera/astrocyte/src-tauri/src/main.rs`)

Message semantics:
  Subsystem bracket tag is first meaningful token inside %(message)s / record.args (prefix narrative duplicate: `17:41:project_chimera/crucible_core/docs/LOGGING_STANDARD.md`)
```

**Not in scope:** log shipping, retention policy, Prometheus — Phase II.D covers metrics JSON separately (`32:34:project_chimera/docs/ROADMAP.md`).

## API / Schema

### Canonical Python formatter

Mandatory for **service + batch scripts** that configure logging at module top (`15:19:project_chimera/crucible_core/scripts/start_oligo.py` canonical block):

| Field | Shape | Meaning |
|-------|-------|---------|
| `format` | `"%(asctime)s \| %(levelname)-8s \| %(name)s \| %(message)s"` | Matches roadmap M3 verbatim (`19:19:project_chimera/docs/ROADMAP.md`) |
| `datefmt` | `"%Y-%m-%d %H:%M:%S"` | Local wall clock, no tz offset in string |
| `level` | typically `logging.INFO` | Scripts may downgrade for dry runs |

**Representative clones:** `run_ingest.py` (`49:53:project_chimera/crucible_core/scripts/run_ingest.py`), `run_daily.py` (`23:26:project_chimera/crucible_core/scripts/run_daily.py`), `run_lens.py` (`56:59:project_chimera/crucible_core/scripts/run_lens.py`), `smoke_task_progress.py` (`55:58:project_chimera/crucible_core/scripts/smoke_task_progress.py`) — same `format`/`datefmt` pair throughout `crucible_core/scripts/`.

### Dev-only exception

`smoke_intent_router_live.py` uses `logging.basicConfig(level=logging.INFO, format="%(message)s")` (`70:70:project_chimera/crucible_core/scripts/smoke_intent_router_live.py`) — **no parity** with M3 columns; acceptable for terse console scraping only.

### Rust (`Astrocyte`) formatter

Custom `writeln!` reproduces Python column order (`10:21:project_chimera/astrocyte/src-tauri/src/main.rs`):

```text
{Local now %Y-%m-%d %H:%M:%S} | {level left 8} | {record.target()} | {args}
```

`record.level()` string is **`WARN` for Rust** vs Python’s **`WARNING`** — accepted mismatch `I.M3.1` (`71:73:project_chimera/docs/ACCEPTED_PARTIALS.md`). Grepping by subsystem still uses **`[Astrocyte]`** / **`[TaskStream]`** prefixes in `args`.

### Prefix → ownership (living vocabulary)

Prefixes are **convention enforced by code review**, not enums. Established emitters:

| Prefix | Typical owner / path | Notes |
|--------|----------------------|-------|
| `[Oligo]` | `server.py`, `agent.py` theater + parser | SSE lifecycle stays `[Oligo]`; **not** `[Stream]` (`34:149:project_chimera/crucible_core/src/oligo/api/server.py`; exemplar `[Oligo]` / `[Router]` / `[Final]` sites `1093:1362:project_chimera/crucible_core/src/oligo/core/agent.py`) |
| `[Router]` | `agent.py` probe / intercept | (`1097:1295:project_chimera/crucible_core/src/oligo/core/agent.py`) |
| `[Final]` | `agent.py` final buffer / watchdog | (`1295:1335:project_chimera/crucible_core/src/oligo/core/agent.py`) |
| `[Tool]` | `agent.py` execution partition; `tool_protocol`; `web_search` | Repairs log at INFO (`594:668:project_chimera/crucible_core/src/oligo/core/agent.py`; `219:219:project_chimera/crucible_core/src/oligo/core/tool_protocol.py`; `76:76:project_chimera/crucible_core/src/oligo/tools/web_search.py`) |
| `[Wash]` | `agent.py` intent wash aggregation | (`951:1275:project_chimera/crucible_core/src/oligo/core/agent.py`) |
| `[Prompt]` | `prompt_composer.py`, `jinja_prompt_manager.py`, router metrics in `agent.py` | (`163:277:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`; `43:64:project_chimera/crucible_core/src/crucible/ports/prompts/jinja_prompt_manager.py`) |
| `[Vault]` | `vault_*` adapters / writer | (`220:392:project_chimera/crucible_core/src/crucible/ports/vault/vault_read_adapter.py`; writer `44:73:project_chimera/crucible_core/src/crucible/ports/vault/vault_note_writer.py`) |
| `[LLM]` | `openai_compatible_client.py` | retries, stream idle (`107:321:project_chimera/crucible_core/src/crucible/ports/llm/openai_compatible_client.py`) |
| `[Config]` | `config.py`, `platform.py` | template write + migration logging (`602:738:project_chimera/crucible_core/src/crucible/core/config.py`; `22:22:project_chimera/crucible_core/src/crucible/core/platform.py`) |
| `[Bootstrap]` | `bootstrap.py` | missing keys / slots (`86:169:project_chimera/crucible_core/src/crucible/bootstrap.py`) |
| `[Notify]` | `telegram_notifier.py` | (`23:89:project_chimera/crucible_core/src/crucible/ports/notify/telegram_notifier.py`) |
| `[Ingest]` | `paper2md.py`, `mineru_pipeline.py` | (`21:114:project_chimera/crucible_core/src/crucible/ports/ingest/paper2md.py`; `178:221:project_chimera/crucible_core/src/crucible/ports/ingest/mineru_pipeline.py`) |
| `[Arxiv]` | `arxiv_fetch.py` | (`61:264:project_chimera/crucible_core/src/crucible/ports/arxiv/arxiv_fetch.py`) |
| `[Service]` | batch / daily / optics / pipeline services | canonical long-form stage logs (`58:147:project_chimera/crucible_core/src/crucible/services/single_paper_pipeline_service.py` sample chain) |
| `[Task]` | `task_service.py` queue backpressure | (`98:103:project_chimera/crucible_core/src/crucible/services/task_service.py`) |
| `[Astrocyte]` | Tauri `lib.rs`, `main.rs`, `memory.rs` | `log!` + `eprintln!` examples (`700:702:project_chimera/astrocyte/src-tauri/src/lib.rs`; `945:962:project_chimera/astrocyte/src-tauri/src/lib.rs`; `1381:1384:project_chimera/astrocyte/src-tauri/src/lib.rs`; `174:178:project_chimera/astrocyte/src-tauri/src/memory.rs`; `394:397:project_chimera/astrocyte/src-tauri/src/memory.rs`; `24:28:project_chimera/astrocyte/src-tauri/src/main.rs`) |
| `[TaskStream]` | `task_stream.rs` EventSource bridge | (`16:66:project_chimera/astrocyte/src-tauri/src/task_stream.rs`) |

**`[Paper]`:** appears only in duplicated legacy doc bullets (`31:31:project_chimera/crucible_core/docs/LOGGING_STANDARD.md`); runtime code favors `[Service]` / `[Vault]` / `[Ingest]` for paper flows — **prefer extending those** before introducing `[Paper]` in new emitters.

## Decision Points

| ID | Decision | Rationale |
|----|----------|-----------|
| M3.A | Columns fixed before FreeBSD-style syslog shipping | Cheap multi-process correlation (`19:19:project_chimera/docs/ROADMAP.md`) |
| M3.B | Bracket subsystem **inside message**, not duplicated `%(name)s` | `%(name)s` retains module path; subsystem tags live in prose (`15:34:project_chimera/crucible_core/docs/LOGGING_STANDARD.md`) |
| I.M3.1 | Accept `WARN` vs `WARNING` string | Avoid custom formatter maintenance (`71:73:project_chimera/docs/ACCEPTED_PARTIALS.md`) |

## Checklist (code-taste gate)

When adding logs:

1. **Python scripts:** replicate `basicConfig` block (`15:19:project_chimera/crucible_core/scripts/start_oligo.py`) unless justified dev smoke (`smoke_intent_router_live.py`).
2. **Message:** prefix with **one** canonical bracket token; avoid `logger.info("[INFO] …")` level echo inside text (`41:41:project_chimera/crucible_core/docs/LOGGING_STANDARD.md`).
3. **Placement:** SSE transport concerns → `[Oligo]`; final buffered assistant text → `[Final]` — do not resurrect `[Stream]` (`39:40:project_chimera/crucible_core/docs/LOGGING_STANDARD.md`).
4. **Rust HUD:** reuse `[Astrocyte]` / `[TaskStream]` patterns; stderr `eprintln!` acceptable for migration / window glue (`1381:1483:project_chimera/astrocyte/src-tauri/src/lib.rs`).
5. **User-facing CLI tables:** framed `print()` in `cli_presenter.py` is **presentation**, not observability (`18:68:project_chimera/crucible_core/src/crucible/services/cli_presenter.py`) — do not replace with noisy loggers unless also human-facing in TTY.

## Known Issues / Drift Watch

- **`[SmokeLive]` prefix** (`41:66:project_chimera/crucible_core/scripts/smoke_intent_router_live.py`) — dev Router smoke harness only; intentionally minimal `logging` wiring (`70:71:project_chimera/crucible_core/scripts/smoke_intent_router_live.py`).
- **`print()` in `agent.py` debug path** dumps raw SSE chunks (`1448:1451:project_chimera/crucible_core/src/oligo/core/agent.py`) — guard behind explicit dev flag if expanded.
- **Uvicorn access logs** use own format; correlate via timestamp + `[Oligo]` app logs, not column identity.
- **`[Paper]`** prefix unused in code paths today — document-only drift vs `crucible_core/docs`.

## Cross-references

- **SSE envelopes** (orthogonal to stderr logs): [`SSE_PROTOCOL.md`](./SSE_PROTOCOL.md)
- **Config knobs touching log_level field:** [`CONFIG_SCHEMA.md`](./CONFIG_SCHEMA.md)
- **[Prompt] / composer budgets:** [`PROMPT_MIDDLEWARE.md`](./PROMPT_MIDDLEWARE.md)
- **[Task] + Astrocyte task channel:** [`TASK_PROGRESS_SYSTEM.md`](./TASK_PROGRESS_SYSTEM.md)

- **`[Tool]` args repair narration:** [`TOOL_PROTOCOL.md`](./TOOL_PROTOCOL.md)
