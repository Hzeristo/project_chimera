# Chimera Roadmap

Personal research OS for one user. Not a framework. Not SaaS.

> **Last sealed:** Phase III.C — Structured Final Contract — 2026-05-25
> **Active:** Phase IV (TBD)

---

## Sealed Phases

### Phase I — Foundation
**Goal:** System never crashes silently. Configuration has single source of truth. Logs are greppable.

| Milestone | Deliverable | Commit |
|---|---|---|
| M1 Error Handling | LLM timeout protection (`generate_raw_text` + outer 120s watchdog), per-tool timeout, `web_search` async via `to_thread`, `BaseException` black hole eliminated, FastAPI streaming exception capture | `{commit}` |
| M2 Configuration Unification | `platform.py` / `platform.rs` cross-language path abstraction, `~/.chimera/config.toml` as single truth, `config.example.toml`, multi-LLM slot config (working/wash/router) | `{commit}` |
| M3 Observability Baseline | Uniform Python log format `%(asctime)s \| %(levelname)-8s \| %(name)s \| %(message)s`, bracket-prefix convention `[Oligo]/[Router]/[Tool]/[Wash]/[Final]/[Vault]/[LLM]/[Config]`, SSE event protocol (`bb-stream-chunk`, `bb-sys-event`, `bb-stream-done`) | `{commit}` |

**Accepted partials:** front-end Provider deletion UI dropped — TOML is authoritative.
**Sealed:** 2026-04-XX.

---

### Phase II — Cognition
**Goal:** System knows what to do with tools, has skills, and we know if any of it is actually used.

| Milestone | Deliverable | Commit |
|---|---|---|
| II.A Skill Ecosystem | Six skill JSONs in `~/.chimera/skills/`, `SkillStatsService` writing `~/.chimera/skill_stats.json`, frontend skill card grid with usage / success-rate / avg-tokens | `{commit}` |
| II.B Tool Registry | `search_vault`, `search_vault_attribute`, `obsidian_graph_query`, `web_search`, `arxiv_miner`, `check_task_status`, `daily_paper_pipeline` registered. Gravedigger deferred to Phase IV. | `{commit}` |
| II.C Wash Refinement | Intent-driven `_wash_tool_result` consuming context window, `_BYPASS_WASH_TOOLS` / `_FORCE_WASH_TOOLS` policy split, dual-engine (cheap wash + premium working) | `{commit}` |
| II.D Observability & Metrics | `MetricsService` with system / tool / wash dimensions, persisted to `~/.chimera/metrics.json`, frontend health panel | `{commit}` |
| II.E Dogfooding (Use Week) | Five friction entries logged. Pattern: all five point to "Astrocyte ↔ CLI workflows not bridged." | `{commit}` |

**Sealed:** 2026-05-XX.

---

### Phase III.A — Connection Convergence
**Goal:** Astrocyte becomes the trigger surface for existing CLI workflows.

| Step | Deliverable | Commit |
|---|---|---|
| Step 0: Contamination Fix (S0.1–S0.4) | `_FINAL_GUARDRAIL` block, Router CMD-syntax relaxation, tool whitelist enforcement, code-block-aware CMD regex pre-strip | `{commit}` |
| Step 1: Boundary Audit | Skill / Tool / Lens conceptual boundary documented. Decision: existing 6 "skills" are *disguised lenses*, retain as legacy markers, no refactor | `{commit}` |
| Step 2: Event-Driven Task Progress | `TaskService.event_queue` + `/v1/tasks/stream` SSE channel, Rust `task_stream.rs` with exponential reconnect, Svelte `ActiveTaskPanel` with 0.1s tabular-num timer using local `Date.now()` basis | `{commit}` |
| Step 3: Daily Pipeline Tool | `daily_paper_pipeline(skip_telegram?)` registered, 4-stage progress emission via `start_stage`, idempotent via `audit_log.csv` | `{commit}` |

**Sealed:** 2026-05-XX.

---

### Phase III.B — Middleware & Harness (Functional Seal)
**Goal:** Prompt assembly stops being string concatenation roulette. Tool calling stops being regex prayer.

| Sub-phase | Deliverable | Commit |
|---|---|---|
| III.B.1 Prompt Middleware (MW.0–4) | `PromptComposer` with 9 registered components, stable/dynamic prefix split via `cacheable` flag, `TextSanitizer` three-layer strip (reasoning / tool-syntax-in-visible / message-history-sanitization), `docs/ARCHITECTURE/PROMPT_MIDDLEWARE.md` | `{commit}` |
| III.B.2 Tool Protocol Lite (TP.0–5) | `ToolRegistry` with `ToolSpec` (concurrency_safe, long_running, args_schema, examples), XML `<tool_call>` parsing alongside legacy `<CMD:...>`, 5-rule argument-repair (code fence / quote style / trailing comma / brace wrap / smart quotes), `partition_tool_calls` by concurrency safety, `docs/ARCHITECTURE/TOOL_PROTOCOL.md` | `{commit}` |
| III.B.3 Intent Recognition (IR.0–5) | Tri-tier tool list rendering (verbose/compact/micro under length budget), `<tool_result status=...>` typed wrapper with reflection hints (failure-only), `EMPTY_RESULT` fallback suggestion, `bb-tool-start`/`bb-tool-done` per-call telemetry, `ActiveToolStrip` 0.1s tool-level timer, `xml_structured` renderer reserved for Phase IV PPR injection, `docs/ARCHITECTURE/INTENT_AND_DEGRADATION.md` | `{commit}` |
| III.B.4 Context-Folding | **Deferred indefinitely.** Friction log shows no demand. May surface in Phase IV trajectory reasoning. |
| III.B.5 Memory CRUD | **Merged into Phase IV.** Will design alongside Exocortex K/T/I/D ontology. |
| III.B.6 UI Visualization | **Folded into Phase III.C.** Attachment rendering and message ops handle the visualization needs. |
| III.C Structured Final Contract (FC.0–6) | `ToolOutput`/`Artifact` for vault tools → `bb-message-artifacts` SSE → Tauri persistence → Svelte chip UI + `open_vault_note`; Router persona-invariance tests; message delete verified. Resolves E3/E4. `docs/ARCHITECTURE/FINAL_CONTRACT.md` | `421a526` |

**Functional seal** rather than full administrative seal — see `ACCEPTED_PARTIALS.md` for trade-offs and `TECHNICAL_DEBT.md` for tracked items.
**Sealed:** 2026-05-25.

---

## Active Phase

# Phase EXT — Prompt Externalization & Router Rewrite

**Status:** Active
**Sealed predecessor:** III.C
**Driving frictions:**
- friction-260526 E1 (router不识别"爬取papers" = daily_paper_pipeline)
- friction-260526 E2 (长任务启动后 Final幻觉工具调用 + 前端无进度)

## Mission

将Oligo 的 prompt 架构从"Python 内联常量"升级为"外部 Jinja2 模板 + 领域化 router prompt",
使 router具备意图识别能力,使 prompt迭代不再需要改Python 代码。

## Sprint Sequence

| Sprint | One-line goal |
|---|---|
| EXT.0 | Audit: 当前 prompt 内联常量清单 +PromptComposer 渲染链路 |
| EXT.1 | 外部化: 所有 inline prompt → .md/.md.j2 文件, 行为不变 |
| EXT.2 | Router prompt 重写: 600tokens → 5500+ tokens, 意图分类 + 推理框架 + 工作示例 |
| EXT.3 | 工具描述 rich化: ToolSpec 加 user_aliases / examples / common_mistakes |
| EXT.4 | Agentic theater讨论(架构决策, 不写代码) |

Dependencies: EXT.0 precedes all. EXT.1 precedes EXT.2/EXT.3.
EXT.4 is a design discussion, not implementation.

## Cross-Sprint Red Lines

- ❌ Do NOT introduce new Python dependencies (Jinja2 already present)
- ❌ Do NOT change PromptComposer's compose() core logic in EXT.1
- ❌ Do NOT modify agent.py theater loop in EXT.1-EXT.3
- ❌ Jinja2 (.md.j2) for static-at-registration templates only;
  runtime-variable templates use plain .md with Python str.format()

## Hard Sealing Conditions

1. Zero inline prompt constants remain in agent.py or prompt_composer.py
   (verified by Grep at seal time)
2. Router prompt contains intent classification framework with≥5 real
   usage examples per tool (verified by reading router template)
3. "爬取论文" as user input correctly triggers daily_paper_pipeline
   (verified by live test or mock)

## Design Decisions (from ST discussion, not re-derivable)

- **Two-phase rendering**: .md.j2 (Jinja2, registration-time) vs .md (str.format, compose-time).Avoids Jinja2/format brace conflict. See ST discussion2026-05-26.
- **Skill injection timing change**: Router gets skill one-line summary only;
  Final gets skill full text. Per Claude Code source analysis (Silver Bullet #1).
- **Router prompt structure**: 7 segments modeled after Anthropic's 16-segment
  architecture, adapted for single-user research OS. See ST discussion for
  segment breakdown.
- **Tool call format**: Keep `<tool_call name="..."><args>...</args></tool_call>`.
  Do NOT adopt Anthropic's ANTML `<function_calls><invoke>` format (provider-agnostic).

## Out of Scope

- Half-blocking long task execution (EXT.4discussion, implementation in later phase)
- Final contamination hard filter (depends on EXT.4 architecture decisions)
- Memory CRUD → Phase IV
- Exocortex → Phase IV

## Completion Signal

用户输入的内容会经过完整的agentic框架，得到充分的意图理解，调用正确的工具。


---

## Queued

### Phase IV — Exocortex & Memory
- K/T/I/D node ontology (Knowledge / Thought / Insight / Decision)
- PaperMiner → Knowledge Node automation (via Lens output → vault note write)
- Shallow PPR retrieval with pruning + fanout activation (star-expansion for hyperedges, no theoretical purity tax)
- Memory CRUD operators (entity + attribute as RDF triple, LLM-as-judge for conflict resolution)
- Gravedigger (OpenReview-Miner with reuse of `FilterService` + `VaultNoteWriter` + `PaperArchiveAdapter`)
- Trajectory reasoning emerges from PPR-tool multi-turn ReAct (no separate infrastructure)

### Phase V — Horizon (speculative)
- Inspiration mechanics (nightly random walk over graph)
- External orchestration (Claude Code as long-task tool, MLLM image gen for slides)
- Self-evolving agent (APO via human-in-the-loop, not full RLAIF)

---

## Status Glossary

- **Sealed:** Phase complete, friction resolved, accepted partials documented.
- **Functional seal:** Phase deliverables work, but some review-time partials remain tracked.
- **Active:** Currently in execution.
- **Queued:** Planned but not started. Scope may shift based on friction logs.
- **Deferred:** Originally planned, now postponed indefinitely. Re-evaluate when triggered by friction.

## Version

This roadmap is updated only at sprint review seal events, by `chimera-sprint-discipline` skill in review mode, with proposed diffs presented for user confirmation.
