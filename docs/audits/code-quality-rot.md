# Code Quality Audit — crucible_core/src/
**Date:** 2026-06-20  
**Scope:** All Python under `crucible_core/src/` (crucible + oligo packages)  
**Method:** Manual full-read, no automated tooling  
**Rule:** Triage only — no fixes applied.

---

## A. Dead Code

### A.1 Unused / Suspect Imports

| File | Line | Symbol | Issue | Fix scope |
|------|------|--------|-------|-----------|
| `crucible/core/schemas.py` | 7–19 | Re-exports from `oligo.core.schemas` | All marked `# noqa: F401` — intentional backcompat shim but bloats the core module and maintains the circular dep (see D.1) | `rewire` |
| `crucible/services/optics_service.py` | 11 | `from openai import APIConnectionError, APIError, APITimeoutError` | `openai` types leaked into services layer; should be caught in the port | `rewire` |
| `oligo/core/agent.py` | 1563–1565 | `from pydantic import BaseModel` + `_HarnessVault` / `MockLLMClient` | Only inside `if __name__ == "__main__"` test harness; carries 200+ LOC of dead weight in the production module | `extract` (move to tests/) |

### A.2 Functions Defined but Never Called (within src/)

| File | Function | Evidence | Fix scope |
|------|----------|----------|-----------|
| `crucible/core/config.py:752` | `load_config()` | Declared "deprecated alias"; no call sites found in src/ | `delete` |
| `crucible/services/optics_lens_registry.py:230` | `_write_default_yaml()` | 2-line wrapper over `_persist_lens_yaml`; only the wrapper is called from `load_lens_configs`, `_persist_lens_yaml` also called directly from `load_survey_lens_configs` — the wrapper adds nothing | `inline` / `delete` |
| `oligo/core/agent.py:1020` | `ChimeraAgent.archive_segment()` | Zero call sites in src/; deferred import inside body suggests prototype; `_archive_log` attr is set but `unarchive_segment` is the only caller and it's also unused | `delete` (or move to experimental/) |
| `oligo/core/agent.py:1054` | `ChimeraAgent.unarchive_segment()` | Only caller of `archive_segment`'s side-effect; itself unused | `delete` |
| `oligo/core/agent.py:1528` | `run_isolated()` | Module-level function; no import sites found in src/ | `delete` or `extract` to tests/ |
| `crucible/services/fetch_arxiv_workflow.py:22` | `run_arxiv_fetch()` | Sync wrapper; async pipeline uses `fetch_and_process_arxiv` instead. Possibly called from `scripts/` (not checked). Flag for verification. | verify → `delete` |

### A.3 Pydantic Fields Never Read (in any call site seen)

| Model | Field | Issue | Fix scope |
|-------|-------|-------|-----------|
| `oligo/core/schemas.py:307` | `TerminalReason` enum | Defined; never referenced in `agent.py` or any route; the agent terminates via SSE yield + return, not this enum | `delete` |
| `oligo/core/schemas.py:316` | `TurnOutcome` enum | Same — `CONTINUE`/`TERMINATE` not used by `_run_theater_stream` or any caller | `delete` |
| `crucible/core/schemas.py:354` | `BatchFilterStats.source_dir` | Set in `run_batch_filter`; never read by any consumer found in src/ | verify → `delete` |
| `crucible/core/schemas.py:282` | `DeepReadAtlas.is_survey` | Set during `irradiate()`; the written atlas note uses other fields; `is_survey` never queried post-write | verify → `delete` |

---

## B. Coupling Smells

### B.1 Cross-Domain Imports

| File | Line | Import | Violation | Fix scope |
|------|------|--------|-----------|-----------|
| `crucible/core/config.py` | 30 | `from src.oligo.core.schemas import OligoAgentConfig` | `crucible/core` (innermost layer) imports from `oligo/core`. **Inverts the declared dependency direction.** `OligoAgentConfig` should live in `crucible/core/schemas` or be injected at bootstrap. | `rewire` |
| `crucible/core/schemas.py` | 7–19 | Re-exports `AgentInvokeRequest`, `ToolOutput`, etc. from `oligo.core.schemas` | `crucible/core` depends on `oligo/core`; this creates a circular risk and violates the crucible-is-foundation rule | `rewire` |
| `oligo/core/agent.py` | 50 | `from src.crucible.services.task_service import get_task_service` | `oligo/core` pulls a service singleton at call time instead of accepting injection; tight coupling to `crucible/services` | `rewire` (inject via constructor or port) |
| `oligo/core/agent.py` | 39 | `from src.crucible.services.metrics_service import MetricsService` | Type-hint only but still couples `oligo/core` to `crucible/services`; MetricsService should be behind a Protocol in `oligo/ports` | `rewire` |
| `oligo/tools/vault_query.py` | 11 | `from src.crucible.core.config import get_config` | Tool calls `get_config()` directly instead of receiving `vault_root` as an injected arg; makes the tool impossible to unit-test in isolation | `rewire` (inject `vault_root: Path`) |
| `oligo/tools/miner_tools.py` | 9–11 | imports from `crucible/services/fetch_arxiv_workflow` and `task_service` | Tools layer reaching directly into crucible application services; should be mediated by a port | `rewire` |
| `crucible/services/filter_service.py` | 10 | `from src.crucible.ports.llm.openai_compatible_client import OpenAICompatibleClient` | Service is typed against the concrete implementation, not the `LLMClient` abstract base from `ports/llm/base.py`; blocks substitution | `rewire` (use `LLMClient` protocol) |
| `oligo/api/staging_routes.py` | 10 | `from src.crucible.core.platform import get_project_root` | API layer hardcodes staging dir relative to project root (see C.1) | `rewire` |

### B.2 God Files (> 800 lines)

| File | Lines | What's crammed in | Fix scope |
|------|-------|------------------|-----------|
| `oligo/core/agent.py` | **1630** | ReAct loop, tool execution, wash logic, render logic, archive prototype, `run_isolated`, 200-line test harness | `extract`: pull test harness to `tests/`, `run_isolated` to utils, `archive_segment` to separate class |

### B.3 God Functions (> 50 lines)

| File | Function | Lines | Fix scope |
|------|----------|-------|-----------|
| `oligo/core/agent.py` | `_run_theater_stream` | ~145 | `extract` per-phase helpers already exist; the loop body can be 20 lines |
| `oligo/core/agent.py` | `_execute_tool_plan_batch` | ~133 | `extract` SSE-emit logic and gather logic into sub-helpers |
| `oligo/tools/registry.py` | `_register_default_tools` | ~317 | Declarative but dense; `extract` each tool spec to its module as a constant |
| `crucible/services/daily_chimera_service.py` | `_run_pipelined_async` | ~132 | `extract` stage coroutines into named functions |
| `crucible/services/batch_filter_workflow.py` | `filter_queue_worker` | ~115 | The stats-update block at lines 196-252 duplicates `run_batch_filter`'s stats block; `extract` shared `_record_verdict_stats()` |
| `crucible/core/config.py` | `_legacy_yaml_to_chimera_nested` | ~108 | One-shot migration function; acceptable complexity but should be moved to a `_legacy.py` sub-module | `extract` |
| `crucible/ports/vault/vault_read_adapter.py` | `_query_graph_sync` | ~97 | `extract` BFS expansion into helper |
| `crucible/services/daily_chimera_service.py` | `_render_daily_report` | ~91 | `extract` inline-keyboard builder |
| `oligo/core/prompt_composer.py` | `_register_default_components` | ~86 | Declarative; low priority | — |
| `oligo/core/agent.py` | `_parse_tool_calls` | ~88 | `extract` allowlist-gate logic |
| `oligo/core/agent.py` | `_step_synthesize` | ~64 | Acceptable; boundary is the prompt-compose + stream loop |

---

## C. Injection Failures

### C.1 Hardcoded Paths Bypassing platform.py / get_config()

| File | Line | Path | Severity | Fix scope |
|------|------|------|----------|-----------|
| `oligo/api/staging_routes.py` | 15 | `_STAGING_DIR = get_project_root() / "docs" / "staging"` | **HIGH** — computed at module load time; `"docs/staging"` not in config; breaks if repo is moved or staging path changes | `rewire` (add `staging_dir` to config or read from `app.state.settings`) |
| `oligo/core/prompt_composer.py` | 22 | `_PROMPTS_DIR = Path(__file__).resolve().parent.parent / "prompts"` | LOW — package-internal file-relative path; acceptable for co-located templates, but not surfaced in config for override | acceptable / low-priority `rewire` |

### C.2 Hardcoded LLM Client Construction Outside bootstrap.py

| File | Line | Issue | Fix scope |
|------|------|-------|-----------|
| `crucible/ports/llm/openai_compatible_client.py` | 96–98 | `OpenAICompatibleClient.__init__` calls `get_config()` directly when `timeout_seconds=None`; port reaching up to the config singleton | `rewire` — caller (bootstrap) should always pass an explicit timeout; make the fallback a module-level constant instead |

### C.3 Hardcoded Model Names / API Endpoints

| File | Lines | Value | Severity | Fix scope |
|------|-------|-------|----------|-----------|
| `crucible/core/config.py` | 195–200 | `"gpt-4o"`, `"deepseek-chat"`, `"claude-3-5-sonnet-20241022"` in `_default_llm_provider_slots()` | MEDIUM — defaults encoded in Python, not in the TOML template; a model deprecation requires a code change | `rewire` (emit defaults only into generated TOML, not hardcode in Python) |
| `crucible/core/config.py` | 233 | `"deepseek-chat"` in `_default_llm_config()` | Same | `rewire` |
| `crucible/core/config.py` | 475 | `"https://api.openai.com/v1"` fallback in `default_llm_base_url` | MEDIUM | `rewire` or document clearly as the intended fallback |
| `crucible/services/daily_chimera_service.py` | 358 | `"https://chimeravaultrouter.haydenshui.workers.dev/..."` Cloudflare Workers URL | MEDIUM — deployment detail should be in config (`system.vault_router_url`) | `rewire` |

---

## D. DDD Layering Violations

Layer contract: `core` ← `ports` ← `services` ← `api/tools`  
`oligo/*` should depend on `crucible/*`, not vice versa.

| # | File | Line | Import | Violation | Severity | Fix scope |
|---|------|------|--------|-----------|----------|-----------|
| D.1 | `crucible/core/config.py` | 30 | `from src.oligo.core.schemas import OligoAgentConfig` | `crucible/core` → `oligo/core`. **Inverts the architectural direction.** `OligoAgentConfig` was migrated to `oligo.core.schemas` but `ChimeraConfig.oligo_agent` still depends on it. This creates a cross-package cycle. | **CRITICAL** | `rewire`: move `OligoAgentConfig` back to `crucible/core/schemas`, or build it inline in `config.py` without importing from oligo |
| D.2 | `crucible/core/schemas.py` | 7–19 | Re-exports from `oligo.core.schemas` | `crucible/core` depends on `oligo/core` to re-export types that used to live there; backcompat shim that preserves the wrong dep direction | HIGH | `rewire`: keep types in `crucible/core/schemas`; migrate `oligo/core/schemas` to import from crucible instead |
| D.3 | `oligo/core/agent.py` | 50 | `from src.crucible.services.task_service import get_task_service` | `oligo/core` (inner) imports `crucible/services` (outer); service should be injected via constructor | HIGH | `rewire` (inject `task_service: TaskService \| None`) |
| D.4 | `oligo/core/agent.py` | 39 | `from src.crucible.services.metrics_service import MetricsService` | `oligo/core` importing from `crucible/services`; MetricsService is injected (good) but import should use a Protocol type in `oligo/ports` | MEDIUM | `rewire` (define `MetricsPort` Protocol in `oligo/ports`) |
| D.5 | `oligo/tools/vault_query.py` | 11 | `from src.crucible.core.config import get_config` | Tool calls config singleton directly | MEDIUM | `rewire` (inject `vault_root: Path`) |
| D.6 | `oligo/tools/miner_tools.py` | 8–11 | `from src.crucible.services.fetch_arxiv_workflow import fetch_and_process_arxiv` | `oligo/tools` (at ports level) reaching into `crucible/services` | MEDIUM | `rewire` (expose via a port callable or inject at lifespan) |
| D.7 | `crucible/services/filter_service.py` | 10 | `from src.crucible.ports.llm.openai_compatible_client import OpenAICompatibleClient` | `services` typed against concrete port impl; blocks substituting a mock/stub | LOW | `rewire` (type hint as `LLMClient` from `ports/llm/base.py`) |
| D.8 | `oligo/api/staging_routes.py` | 10 | `from src.crucible.core.platform import get_project_root` | API layer using platform directly to compute a hardcoded staging path | LOW | `rewire` (see C.1) |

---

## Fix Plan

### Tier 1 — Pure Wins (Effort ≤ Small, Value ≥ Medium, Risk = Zero)

| # | What | Files | Effort |
|---|------|-------|--------|
| T1.1 | Extract test harness from `agent.py` → `tests/oligo/test_agent_harness.py`; delete lines 1563–1765 | `oligo/core/agent.py`, `tests/oligo/test_agent_harness.py` (new) | ~10 min |
| T1.2 | Delete dead functions: `archive_segment`, `unarchive_segment`, `run_isolated`, `load_config` (~90 LOC) | `oligo/core/agent.py`, `crucible/core/config.py` | ~5 min |
| T1.3 | **C.1** — `_STAGING_DIR`: add `staging_dir: Path` to `SystemConfig` (default `project_root / "docs" / "staging"`); read in `staging_routes._svc()` from `settings` | `crucible/core/config.py`, `oligo/api/staging_routes.py` | ~15 min |
| T1.4 | **C.3** — Cloudflare URL: add `vault_router_url: str | None` to `SystemConfig`; read in `_render_daily_report` | `crucible/core/config.py`, `crucible/services/daily_chimera_service.py` | ~5 min |

### Tier 2 — Medium Wins (Effort ≤ Medium, Value ≥ High, Risk ≤ Low)

| # | What | Files | Effort |
|---|------|-------|--------|
| T2.1 | **B.3 dedup** — extract shared `_record_verdict_stats(stats, paper, result, output_path)` from the identical ~50-line block duplicated in `filter_queue_worker` and `run_batch_filter` | `crucible/services/batch_filter_workflow.py` | ~20 min |
| T2.2 | Delete `TerminalReason` / `TurnOutcome` from `oligo/core/schemas.py` (dead enums, zero callers) | `oligo/core/schemas.py` | ~5 min |

### Tier 3 — Defer to Architecture Sprint (Effort ≥ Large or requires design decision)

| Item | Blocker |
|------|---------|
| **D.1 + D.2**: `crucible/core` ← `oligo/core` cycle | Needs a decision on where `OligoAgentConfig` lives permanently. |
| **D.3**: Inject `task_service` into `ChimeraAgent` | Requires threading through `agent_invoke` → lifespan; medium coupling change. |
| **B.3**: Refactor `_run_theater_stream` (145 LOC god function) | Low defect risk; Phase VI async 改造时顺手做。 |

---

## Rebuttals

**T1.2 — include `load_config()` in the deletion pass.** The deprecated alias at `config.py:752` is the same zero-callers dead code as `archive_segment`; it was omitted from the tier list but belongs in the same 5-minute pass. Added above.

**T1.3 — default should not be `None`.** `staging_dir` should default to `project_root / "docs" / "staging"` (mirroring current hardcode), not `None`. A `None` default pushes a guard onto every call site and makes the field behave differently from every other `SystemConfig` path. Use `Optional` only if the staging feature is meant to be explicitly disableable; otherwise give it the derived default.

**D.1 deferral is correct — add a marker now.** Before the architecture sprint, add `# ARCH-DEBT D.1` at the two import sites (`config.py:30`, `schemas.py:7`) so the cycle is visible during grep/search. Zero effort, prevents it from being forgotten.
