# Audit: Chimera (oligo) → Claude Code Migration Feasibility

**Date:** 2026-06-27
**Scope:** Can the Claude Code harness replace oligo's custom ReAct agent loop?
**Method:** Read `crucible_core/src/oligo/core/agent.py` in full (1485 lines) plus the
infrastructure it depends on (`task_service.py`, `tools/registry.py`, `core/sse.py`).
Q2 is grounded in the actual Claude Code harness, not assumptions.

**Verdict up front:** The *tools* migrate cleanly to MCP and would be *better* there.
The *agent loop* is replaceable and would shed ~2000 lines of fragile code. But oligo
is the streaming backend for a **custom Rust frontend (astrocyte)** and a **persona
"theater" UX**, and Claude Code **cannot drive either**. Migration is only correct if
the user is willing to retire astrocyte and the BB persona. While astrocyte lives, a
full loop migration is the wrong call — see Q4/Q5.

---

## Q1 — oligo's core execution model

`ChimeraAgent.run_theater()` → `_run_theater_stream()` runs a bounded ReAct loop
(`max_turns=5`). Each turn passes through an explicit phase state machine
(`AgentPhase`), emitted to the client as `bb-phase-transition` SSE events.

### Per-turn phases

| Phase | Method | What it does |
|---|---|---|
| ROUTING | `_step_route` | **Silent probe.** On turn >1 swaps system[0] to a continuation prompt. Sanitizes history, calls `router_client.generate_raw_text` **non-streamed** (120s watchdog). Strips reasoning tags, parses tool calls from text. Returns `RouteResult{planned_calls, wash_context, is_trivial}`. |
| EXECUTING | `_step_execute` → `_execute_tool_calls` | Partitions allowed calls into batches by `concurrency_safe`. Detects long-running tools, extracts an 8-hex `task_id` from the result. |
| AWAITING_TASK | inline in `_run_theater_stream` | Only if a long-running tool fired. `TaskService.await_completion(task_id)` — event-driven wait (600s), `asyncio.gather` across all pending tasks. Patches results to COMPLETED/FAILED. |
| WASHING | `_step_wash` → `_wash_tool_results` | Per-tool policy: `bypass_wash_tools` → raw; `force_wash_tools` + `wash_min_chars` → cheap-model "Cognitive Filter" compression; else raw. Accumulates artifacts (deduped by `(kind, path)`). |
| RENDERING | `_step_render` | Appends assistant msg (CMD-only extract) + user msg (`[SYSTEM TOOL RESULTS]` XML with status/reason taxonomy + reflection hints). `continue` → next turn. |
| SYNTHESIZING | `_step_synthesize` | Only if **no** tool calls. Backfills probe draft, then **late persona bind**: discards router system, builds Final system (L1 core+skill, L2 persona, L3 author's note, guardrail), calls `llm_client` **non-streamed**, then **fake-streams** the buffer. |

### How it calls tools
Not native function-calling. The router model emits a **text DSL** (`<tool_call
name=...><args>{...}</args></tool_call>` or `<CMD:...>`). `_parse_tool_calls` strips
markdown code blocks (so examples aren't executed), parses with conservative arg-repair,
validates tool name against `TOOL_REGISTRY` and `allowed_tools`, materializes
`PlannedToolCall`. Dispatch is a `dict[str, async-callable]` lookup.

### How it handles async (AWAITING_TASK)
Long-running tools (`arxiv_miner`, `daily_paper_pipeline`) return a `task_id`
**immediately**. `TaskService` runs the real work as a detached `asyncio.create_task`,
persists state to `~/.chimera/tasks/{id}.json`, and publishes `TaskEvent`s through an
in-process pub/sub queue. The agent then **suspends the same HTTP request** on
`await_completion` (subscribe + double-checked terminal status to avoid the
subscribe-after-emit race), and resumes the loop when the COMPLETED event lands —
**within the same streaming response.**

### How it emits streaming output (SSE)
Two frame families over one SSE response:
- **Named events** (`sse_event`): `bb-phase-transition`, `bb-tool-start`/`bb-tool-done`,
  `bb-stream-chunk`, `bb-message-artifacts`, `bb-stream-done`.
- **Telemetry `data:` frames** (`__SYS_TOOL_CALL__` + JSON): router decisions, wash
  stats, per-tool execution status.

Crucial detail: **the router probe and the final answer are both generated
non-streamed (fully buffered), then the final text is artificially chunked** into
3-char slices with `await asyncio.sleep(0.04)` between them. This is deliberate —
the "silent reasoning + ultimate stream" design hides the tool DSL from the user and
presents only a clean persona-voiced answer. Tool-execution SSE is bridged to the
output stream concurrently via an `asyncio.Queue` drained while `_step_execute` runs.

---

## Q2 — Claude Code's agent loop (grounded in this harness)

| Concern | Claude Code behavior |
|---|---|
| Tool calls | **Native API tool-calling.** Independent calls emitted in parallel in one assistant turn; dependent calls sequenced; chained across turns. No text-DSL, no arg-repair, no code-block stripping — the API returns structured tool-use blocks. |
| Long-running tasks | **No in-turn coroutine suspension.** Background work (`Bash run_in_background`, `Task*` tools, `Workflow`) runs detached; the **harness re-invokes me in a new turn when it completes**. A blocking tool (incl. an MCP call) can also just take a while and return synchronously. There is no "suspend this streaming response and resume it" primitive. |
| Streaming | **Real token streaming** to terminal/IDE as I generate. Not buffer-then-dribble. The protocol is fixed by the harness; I cannot define custom event names. |
| Custom context | **Skills** (markdown, injected on trigger), **MCP tools** (schemas fetched on demand via ToolSearch), **subagents/workflows**, `CLAUDE.md`. Persona/system via system prompt + skills, applied uniformly across the turn (no mid-turn system swap). |

---

## Q3 — Capability delta (specific)

| oligo capability | Verdict | Why |
|---|---|---|
| **ReAct loop** (route → tool → synthesize) | ✅ native (loop) / ❌ (exact structure) | The *loop* IS Claude Code's native behavior. But oligo's **two-model split** (a dedicated non-streamed *router* model that decides tools, then a *separate* late-bound *persona* model for the answer) is not how the harness works — one model, one streamed pass. The outcome is reproducible; the architecture is not. |
| **Parallel tool dispatch via `asyncio.gather`** | ✅ native / ⚙️ for the *contract* | Claude Code dispatches independent tool calls in parallel out of the box. But oligo's parallelism is **declaratively gated**: `partition_tool_calls` serializes any tool with `concurrency_safe=False` (`arxiv_miner`, `daily_paper_pipeline`, `fork_agent`) and only batches safe ones. Claude Code's parallelism is **model judgment, not a hard guarantee**. To enforce "never run this tool concurrently," push the lock into the MCP server. |
| **Long-running await** (AWAITING_TASK) | ⚙️ achievable, different semantics / ❌ for the exact UX | An MCP tool can block until done, or return a `task_id` that I poll in a later turn — both work. But oligo's specific behavior — **suspend the live response, await a 10-min job, stream its stage-progress on the same SSE, then continue reasoning in the same turn** — is ❌. Claude Code splits that across turns (background → re-invoke). Functionally equivalent; experientially different. |
| **SSE custom protocol** (`bb-stream-chunk`, `bb-message-artifacts`, `bb-phase-transition`) | ❌ Claude Code cannot | The harness owns the wire format and renders to terminal/IDE. I cannot emit `bb-*` named events to an external consumer. **Anything downstream that parses these events (astrocyte) cannot be driven by Claude Code.** Hard no. |
| **Persona binding** (Final-stage late persona injection) | ⚙️ effect yes / ❌ mechanism | Skills + `CLAUDE.md` + system prompt give a persona-voiced answer. But oligo's **two-phase system swap** (neutral router system during tool selection → persona system for the answer only, so the tool machinery is never "in character" and never visible) has no analog — Claude Code uses one system context for the whole turn and streams the work transparently. |

---

## Q4 — What would be lost (the honesty test)

Migration is **not** free. Things oligo does that Claude Code would do worse or not at all:

### 1. The custom frontend (astrocyte) loses its backend — HARD LOSS
oligo exists to feed a Rust frontend that consumes the `bb-*` event vocabulary: phase
transitions drive a UI state machine, `bb-tool-start/done` render tool chips,
`bb-message-artifacts` render attachments, `bb-stream-chunk` paints the answer.
Claude Code's output is **not an SSE endpoint you can point a browser/Rust client at** —
it streams to a terminal/IDE. There is no adapter that turns my stream into the `bb-*`
protocol. **Migrating the loop means retiring astrocyte or rebuilding it against
something else.** Given astrocyte was retheme'd as recently as commit `c17b024`, this is
a live, valued surface — not dead code. This single fact dominates the decision.

### 2. The "theater" persona UX is a philosophy mismatch — REAL LOSS
oligo deliberately **hides the machinery**: it reasons about tools non-streamed so the
DSL never leaks, then streams only a clean, in-character answer. Claude Code's design is
the opposite — **show your work**: tool calls and reasoning stream transparently. For a
companion/persona product ("BB"), Claude Code's transparency is a downgrade you cannot
configure away. This is design intent, not a missing feature.

### 3. Deterministic tool-dispatch ordering — CONTROL LOSS (recoverable)
oligo *guarantees* unsafe tools never run concurrently. Claude Code's parallelism is
heuristic. For a pipeline that writes the vault/disk, accidental concurrency risks state
corruption. Recoverable by moving the lock into MCP servers — but the guarantee moves
from "the loop enforces it" to "every tool author must remember to."

### 4. Intent-driven Wash — CONTROL LOSS (recoverable)
oligo runs a **cheap second model** to compress noisy tool output against router intent
before feeding it back, with precise per-tool policy (`bypass`/`force`/threshold).
Claude Code manages context automatically (auto-compaction) but exposes **no hook** to
insert a custom per-tool cheap-model filter. Recoverable by having MCP tools return
pre-washed output — but you lose the *intent-aware* dimension (the wash sees the
conversation context; an MCP tool does not).

### 5. In-request progress for long jobs — UX LOSS
The "single response that shows a 10-minute pipeline progressing, then keeps reasoning"
is gone. Claude Code does background-then-re-invoke. The work completes; the *continuous
narrative* does not.

### Honesty cuts both ways — what Claude Code does BETTER
- **Native tool-calling deletes the most fragile code in the repo.** `tool_protocol`,
  `parse_args_with_repair`, markdown-code stripping, malformed-name guards, the entire
  `<CMD>`/`<tool_call>` DSL and its sanitizers exist *only* because oligo parses tools
  out of free text. Native function-calling makes all of it unnecessary. This is the
  strongest pro-migration argument.
- **Real streaming vs. fake streaming.** oligo's "stream" is `buffer → 3-char slices →
  sleep(0.04)`. Claude Code streams actual tokens.
- **Context management is automatic** — no hand-rolled wash budget to maintain.

If this audit had concluded "nothing would be lost," it would be wrong. The losses are
concentrated in the **frontend + persona experience**, and they are real.

---

## Q5 — Migration architecture (minimal Claude Code + MCP)

### KEEP — migrate to MCP servers
| Component | Notes |
|---|---|
| Vault tools (`search_vault`, `read_vault_file`, `vault_query`, `obsidian_graph_query`, `search_vault_attribute`) | Pure domain logic → MCP tools, ~1:1. |
| PaperMiner (`arxiv_miner`, `daily_paper_pipeline`) | Long-running → MCP tool returns `task_id`; poll via a `check_task_status` MCP tool in a later turn. |
| `TaskService` | Keep as the long-running backend behind the MCP tools (poll model, not in-request await). |
| `web_search` | Or drop in favor of Claude Code's native WebSearch. |

### DELETE — Claude Code replaces
| Component | Replaced by |
|---|---|
| `ChimeraAgent` / `_run_theater_stream` and all `_step_*` | Native agent loop |
| `tool_protocol`, `parse_args_with_repair`, `TextSanitizer` CMD-stripping | Native tool-calling (no DSL to parse/strip) |
| `core/sse.py` + `bb-*` protocol | Native streaming |
| `prompt_composer` router/final staging, late persona bind | System prompt + skills |
| Wash machinery (`_wash_tool_results`, Cognitive Filter) | Auto context management (or pre-wash inside MCP) |

### REWRITE — different paradigm
| Component | Outcome |
|---|---|
| `chimera-*` skills | Already Claude Code skills — mostly compatible, minor tuning. |
| Prompts (`system_core`, `persona`, `authors_note`) | → `CLAUDE.md` + system prompt. **The BB persona-theater UX cannot be fully reproduced.** |
| Phase/sprint/audit docs | Unaffected — stay as docs. |
| **astrocyte (Rust frontend)** | **Retire or rebuild.** Cannot consume Claude Code's stream. **This is the crux: if astrocyte must live, do not migrate the loop.** |

---

## Q6 — Migration cost vs. maintenance cost (honest person-days, single dev)

### Migration effort
| Task | Days |
|---|---|
| MCP server: vault tools (5) | 2–3 |
| MCP server: miner + TaskService long-running (poll semantics) | 3–4 |
| Skill/prompt port + `CLAUDE.md` persona | 1–2 |
| Parity tests + HSC smoke verification | 2–3 |
| **Core subtotal (astrocyte retired)** | **~8–12** |
| Frontend rebuild *if astrocyte must be preserved* | **+10–20+** |

### Ongoing maintenance if NOT migrated
| Liability | Burden |
|---|---|
| Agent-loop bugs (1485 lines of async/SSE/state-machine) | Recurring; highest single liability |
| `tool_protocol` arg-repair | Structurally fragile — every model quirk = a patch |
| Client-disconnect / pipe-broken / SSE edge cases | Ongoing |
| Frontend protocol coupling (every new `bb-*` event touches both sides) | Per-feature tax |

### The judgment
This is a **single-user personal research OS** (per `CLAUDE.md`: anti-framework,
friction-driven, do-not-generalize). The decision reduces to **one question**:

> Does the user value the **astrocyte frontend + BB persona experience**, or only
> **getting research tasks done via tools**?

- **If only the tools matter:** Claude Code is strictly better. It deletes the repo's
  most bug-prone ~2000 lines (DSL parsing, SSE, wash, loop), and the tools become clean
  MCP servers. Migrate. ~8–12 days, large maintenance dividend.
- **If the frontend/persona matter** (and the recent astrocyte retheme suggests they
  do): a full loop migration **loses the product**, because Claude Code cannot speak the
  `bb-*` protocol or run the silent-persona theater. **Do not migrate the loop.**

**Recommended path (hybrid, low-regret):** Decouple the tools into MCP-ready modules
*now* — this is good hygiene regardless and lets oligo, Claude Code, *or* a future
frontend call them. **Keep the oligo loop + astrocyte** as long as the persona UX is
valued. Treat "delete the custom loop" as a separate decision gated entirely on whether
astrocyte is retired — not bundled with the tool refactor.

The migration is **technically feasible and would improve code health**, but it is **not
a like-for-like replacement**: it trades a bespoke persona/streaming product for a
transparent tool-runner. That trade is the user's to make, and it hinges on astrocyte,
not on the loop.
