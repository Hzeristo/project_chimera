# Chimera Roadmap

Personal research OS for one user. Not a framework. Not SaaS.

> **Last sealed:** Phase III.B.3 (functional seal) — 2026-05-XX
> **Active:** Phase III.C — Structured Final Contract (planning complete, FC.0 audit ready)

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

**Functional seal** rather than full administrative seal — see `ACCEPTED_PARTIALS.md` for trade-offs and `TECHNICAL_DEBT.md` for tracked items.
**Sealed:** 2026-05-XX.

---

## Active Phase

### Phase III.C — Structured Final Contract
**Goal:** Tool results carry structure. Final outputs carry attachments. Persona stops contaminating Router. Messages can be deleted.

**Driving frictions:**
- E3 (tool results unlinkable, persona contamination) — root causes spread across FC.1, FC.2, FC.3, FC.4
- E4 (cannot delete chat messages) — FC.5

**Sprint sequence:**

| Sprint | Status | Scope |
|---|---|---|
| FC.0 Audit | Ready | Tool return shape, render path, frontend message schema, persona injection timing |
| FC.1 Structured `ToolOutput` | Pending | `ToolOutput` / `Artifact` Pydantic models, vault tools opt-in, `ExecutedToolResult.artifacts` field |
| FC.2 Artifact Pipeline | Pending | Aggregate + dedupe across turns, `bb-message-artifacts` SSE event before `bb-stream-done`, Rust forwarding, persistence |
| FC.3 Attachment UI | Pending | `MessageAttachments.svelte`, vault_note chip → Obsidian open, design-token compliance |
| FC.4 Persona / Router Separation | Pending | `persona_override` stage = FINAL only, Router behavior persona-invariant (asserted by test) |
| FC.5 Message Delete | Pending | `delete_message` Tauri command, hover action, user/assistant scope only |
| FC.6 Docs & E2E | Pending | `FINAL_CONTRACT.md`, `smoke_structured_tool_output.py`, doc updates |

**FC.4 may run in parallel** with FC.1–FC.3 (touches different layer).

**Phase red lines:**
- ❌ Artifacts MUST NOT enter LLM messages or prompt
- ❌ No new dependency on OpenAI `response_format` / function calling APIs
- ❌ No persona-side prompt mutation observable to Router

**Completion signal:** User clicks attachment chip on BB's reply, Obsidian opens the actual note. If yes, sealed.

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
