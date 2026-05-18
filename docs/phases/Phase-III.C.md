# Phase III.C — Structured Final Contract

**Status:** Active (FC.0 audit ready, FC.1–FC.6 pending)
**Started:** 2026-05-XX
**Driving frictions:** E3 (3a, 3b, 3c subroots), E4

---

## Goal

Tool results carry structure. Final outputs carry attachments. Persona stops contaminating Router. Messages can be deleted.

After this phase, the user clicks a chip on BB's reply and Obsidian opens the actual note. That is the seal criterion.

---

## Scope Decisions (set during planning)

- ✅ In scope: structured `ToolOutput`, artifact pipeline, frontend chip rendering, persona/router separation, message delete.
- ❌ Out of scope: PPR retrieval (Phase IV), Memory CRUD (Phase IV), Context-Folding (deferred), trajectory reasoning (emergent in Phase IV).

---

## Sprint Sequence

### FC.0 — Audit (read-only, prerequisite)
**Goal:** Document current tool return shapes, render path, frontend message schema, persona injection timing with `file:line` references.

**Files in scope:**
- `src/oligo/core/agent.py` (tool execution + render path)
- `src/oligo/tools/*` (all tool return types)
- `src/oligo/api/server.py` (SSE assembly)
- `astrocyte/src/routes/+page.svelte` (Message type, render)
- `astrocyte/src-tauri/src/llm_client.rs` (event forwarding)
- `astrocyte/src-tauri/src/state.rs` (Message struct)
- `src/oligo/core/prompt_composer.py` (persona stage assignment)

**Deliverable:** Markdown table answering Q1–Q7 from skill prompt with file:line evidence. No code modifications.

---

### FC.1 — Structured `ToolOutput` (backend)
**Goal:** Vault tools opt-in to return `ToolOutput(text, artifacts)` instead of bare `str`.

**Files modified:**
- `src/crucible/core/schemas.py` (~30 lines new: `ToolOutput`, `Artifact`, `ArtifactKind`)
- `src/oligo/tools/vault_tools.py` (~25 lines: `search_vault`, `search_vault_attribute`, `read_vault_file` return `ToolOutput`)
- `src/oligo/core/agent.py` (~10 lines: `ExecutedToolResult.artifacts` extension)

**Acceptance:**
- `search_vault("query")` returns `ToolOutput.artifacts` with `len ≥ 1` for known query
- All `Artifact.path` values pass `Path(p).exists()`
- `pytest tests/oligo/test_vault_tools.py -x` passes
- Existing str-returning tools (web_search, arxiv_miner, check_task_status, daily_paper_pipeline, obsidian_graph_query) unchanged

**Red lines:**
- ❌ Artifacts MUST NOT enter `_render_tool_results_for_llm` output
- ❌ No new dependencies
- ❌ `ExecutedToolResult` existing fields unchanged (extension only)
- ❌ No modifications to non-vault tools

---

### FC.2 — Artifact Pipeline (backend → SSE → Tauri)
**Goal:** Aggregate artifacts across multi-turn tool calls; deliver to frontend via independent SSE event before stream-done.

**Files modified:**
- `src/oligo/core/agent.py` (~20 lines: session-scope artifact accumulation + dedup)
- `src/oligo/api/server.py` (~10 lines: emit `bb-message-artifacts` before `bb-stream-done`)
- `astrocyte/src-tauri/src/llm_client.rs` (~15 lines: forward as Tauri event)
- `astrocyte/src-tauri/src/state.rs` (~5 lines: `Message.artifacts` field)

**Acceptance:**
- After single `search_vault` call, frontend `Message.artifacts` non-empty
- After session reload, artifacts persist
- LLM `messages` array contains zero artifact references (verified via grep)

**Red lines:**
- ❌ Artifacts MUST NOT appear in any `messages` payload to LLM
- ❌ `bb-message-artifacts` MUST fire before `bb-stream-done`
- ❌ Empty artifacts → no event (don't pollute SSE stream)

---

### FC.3 — Attachment UI (frontend)
**Goal:** Render artifacts as clickable chips below assistant messages. Click opens in Obsidian (or system default).

**Files modified:**
- `astrocyte/src/lib/MessageAttachments.svelte` (new, ~80 lines)
- `astrocyte/src/routes/+page.svelte` (~5 lines: import + integration)
- `astrocyte/src/app.css` (~25 lines: chip styling using existing tokens)
- `astrocyte/src-tauri/src/lib.rs` (~20 lines: `open_vault_note` command with vault_root boundary check)

**Acceptance:**
- Vault note chip renders with title + path tooltip
- Click triggers Obsidian via `obsidian://` URI; fallback to system default `.md` opener
- Multiple artifacts wrap correctly
- Visual cohesion with `ActiveTaskPanel` and stage cards

**Red lines:**
- ❌ No new color tokens (use existing palette)
- ❌ No emoji as UI icons
- ❌ `Message.artifacts` remains optional (not required)
- ❌ `open_vault_note` MUST verify path is within vault_root

---

### FC.4 — Persona / Router Separation (parallel-eligible)
**Goal:** `persona_override` injects only at FINAL stage. Router behavior is persona-invariant.

This sprint can run in parallel with FC.1–FC.3 since it touches `prompt_composer.py` and tests, not tool/SSE/UI layers.

**Files modified:**
- `src/oligo/core/prompt_composer.py` (~5 lines: change `final_persona_override` stage if not already FINAL-only; verify others)
- `src/oligo/core/agent.py` (~3 lines if any persona-related concatenation in Router path remains)
- `tests/oligo/test_prompt_composer.py` (~30 lines new test: persona-invariance assertion)

**Acceptance:**
- Unit test: identical `messages` + different persona → identical Router probe output (mocked LLM with deterministic response)
- E2E smoke: persona = "Reviewer Zero", `search_vault` call → final response not containing fabricated note content

**Red lines:**
- ❌ No persona file deletions
- ❌ FINAL behavior must remain persona-aware (only Router becomes persona-blind)
- ❌ No new `PromptStage` enum values

---

### FC.5 — Message Delete (UI)
**Goal:** User can delete messages via hover action.

**Files modified:**
- `astrocyte/src-tauri/src/lib.rs` (~25 lines: `delete_message` command)
- `astrocyte/src-tauri/src/state.rs` (~10 lines: session entry removal logic)
- `astrocyte/src/routes/+page.svelte` (~30 lines: hover action, delete handler)

**Acceptance:**
- Hover over user/assistant message → delete button appears
- Click → message removed from view
- App restart → message remains deleted
- system / tool / stage_card entries cannot be deleted (permission denied)

**Red lines:**
- ❌ Delete is destructive (no undo in this sprint, defer to Phase V)
- ❌ Must not break audit log if memory persistence enabled
- ❌ stage_card / tool_strip rows are not "messages," not deletable

---

### FC.6 — Documentation & E2E
**Goal:** `FINAL_CONTRACT.md` documents the complete pipeline. Smoke test verifies end-to-end.

**Files created:**
- `docs/ARCHITECTURE/FINAL_CONTRACT.md` (~150 lines)
- `scripts/smoke_structured_tool_output.py` (~80 lines)

**Files updated:**
- `docs/ARCHITECTURE/TOOL_PROTOCOL.md` (add `ToolOutput` to checklist)
- `docs/ARCHITECTURE/PROMPT_MIDDLEWARE.md` (note `final_persona_override` stage change)

---

## Phase Red Lines (apply to all sprints)

- ❌ Artifacts MUST NOT enter LLM messages or prompt context
- ❌ No new dependency on OpenAI `response_format` / native function-calling APIs
- ❌ No persona-side prompt mutation observable to Router
- ❌ No new persistence files (artifacts use existing session persistence)
- ❌ No regression of III.B.1 / III.B.2 / III.B.3 acceptance criteria

---

## Completion Signal

User clicks attachment chip on BB's reply, Obsidian opens the actual note.

If yes → seal phase, append commit to `ROADMAP.md`, append accepted partials, file remaining items as debt.
If no → investigate, do not seal regardless of test pass status.

---

## Post-Phase Plan

After III.C seal:
1. **Use Week #2** — verify FC.1–FC.5 actually resolves Entry E3 in real research workflow
2. Friction log review → decide Phase IV scope (Exocortex vs other priorities)

---

*This file is the working canvas for III.C. Sprint progress (✅/⏳/❌) updates inline. On phase seal, this file is archived under `docs/phases/sealed/phase-III.C.md` and `ROADMAP.md` records the seal commit.*
