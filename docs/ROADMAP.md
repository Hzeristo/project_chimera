# Chimera Roadmap

Personal research OS for one user. Not a framework. Not SaaS.

> **Last sealed:** Phase III.E — Oligo Orchestration Primitives — 2026-06-11
> **Active:** Phase IV — Exocortex & Memory

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

### Phase III.B & C — Middleware & Harness (Functional Seal)
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

---

### Phase EXT — Prompt Externalization & Router Rewrite

**Goal:** Upgrade Oligo prompt architecture from inline Python constants to external Jinja2 templates; give Router intent-classification capability.

| Sprint | Deliverable | Commit |
|---|---|---|
| EXT.0 | Inline prompt audit + PromptComposer render chain map | — |
| EXT.1 | All inline prompts → .md/.md.j2, behaviour unchanged | `10a282a` |
| EXT.2a–2d | 4000-char cap removed; router_intro.md.j2 verified; router_continuation.md.j2 + turn-based system prompt switch; `<thinking>` strip | `d458009`, `b757ced` |
| EXT.3 | ToolSpec enriched: user_aliases / examples / common_mistakes | `a79fa0f` |
| EXT.4 | Agentic theater design discussion — resolved, no code needed | — |

**Sealed:** 2026-06-11 (condition 3 waived by user).

---

### Phase III.E — Oligo Orchestration Primitives

**Goal:** Upgrade Oligo from single-thread theater to multi-context orchestrator; three new primitives as Phase IV prerequisites.

| Sprint | Deliverable | Commit |
|---|---|---|
| III.E.0 | Phase audit: context accumulation points + ChimeraAgent instantiation chain | — |
| III.E.A | `fork_subagent` + `run_isolated`: isolated child agent, reuses wash/router clients, budget conservation, returns summary ≤ 4096 chars | `b40d69f` |
| III.E.C | `archive_segment` / `unarchive_segment`: tombstone replaces segment in `self.messages`, original persisted to `~/.chimera/archive_log/` | `b40d69f` |
| III.E.B | `TaskService.run_subprocess_task`: asyncio subprocess, stdout → `emit_stage_progress`, stall detection, exit≠0 → lesson via backward trace | `b40d69f` |
| — | `fork_agent` registered as tool stub (e2e wiring deferred to Phase IV) | `b40d69f` |

**HSC verification:**
- HSC 1 (downgraded): 50K-token prompt via `fork_subagent` stays out of parent messages — unit test
- HSC 2: `archive_segment` → tombstone in live context, recoverable via `unarchive_segment` — manually verified 2026-06-11
- HSC 3: `run_subprocess_task` shows live progress in `ActiveTaskPanel`, produces DEAD_END lesson on failure — manually verified 2026-06-11

**Sealed:** 2026-06-11.

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
