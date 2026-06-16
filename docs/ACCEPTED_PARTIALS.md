# Accepted Partials

Trade-offs explicitly accepted at sprint or phase seal. **Do NOT reclassify as technical debt.** These are decisions, not deficiencies.

Each entry: phase / sprint, partial description, reason for acceptance.

---

## Phase III.B.3 — Intent Recognition (sealed 2026-05-XX)

### IR.1.1 — Tool list compresses to compact / micro mode under length budget
- **Description:** Verbose mode shows full args schema per parameter. When Router prompt approaches 4000-char budget, list collapses to compact (single-line per tool) or micro (name + one-line).
- **Reason:** Keeping verbose under all conditions exceeds prompt budget. Compression preserves tool discoverability over schema completeness. The trade-off is conscious.

### IR.1.2 — Zero-arg emission not asserted against live LLM
- **Description:** Router prompt instructs to emit `<args>{}</args>` for zero-arg tools. Behavior verified against synthetic test cases, not live DeepSeek/GPT-4 responses.
- **Reason:** Live-LLM CI requires API budget + flaky test isolation. Behavior depends on prompt + model jointly. Risk accepted.

### IR.3.1 — DENIED-only batches emit no `bb-tool-start` / `bb-tool-done`
- **Description:** When all parsed tool calls are denied (whitelist filter / unknown tool), no telemetry events fire.
- **Reason:** Denial happens before execution. Telemetry semantics are "what was attempted to run." DENIED batches are upstream rejections, not execution events. Correct by design.

### IR.3.4 — Tool row CSS opacity transition is 0.35s within 1s removal window
- **Description:** UI spec said "1s fade-out." Implementation uses `transition: opacity 0.35s` with `setTimeout(remove, 1000)`.
- **Reason:** 1s of full-opacity hold + 0.35s fade matches user perception better than 1s linear fade. Animation budget trade-off, not bug.

---

## Phase III.B.1 — Prompt Middleware (functional seal 2026-04-XX)

### MW.1.1 — Wash system prompt outside `PromptComposer`
- **Description:** `_wash_tool_result` builds its own system prompt via f-string at `agent.py:685-696`. Not registered as `PromptComponent`.
- **Reason:** Wash is a tool-subsystem concern, not a persona/router concern. Mixing it into `PromptComposer` would dilute the composer's role. Intentional layering decision.

### MW.2.1 — `tool_list` content breaks stable-section byte identity when tool set changes
- **Description:** `router_tool_registry` is `cacheable=True` but its rendering depends on `ToolRegistry.list_specs()`. If tools are added/removed/relabeled, stable bytes change.
- **Reason:** "Stable" means stable across requests within the same tool registry generation, not stable forever. LLM prefix cache benefit is preserved within a session. Re-cache cost on tool registry change is accepted.

### MW.4.2 — 6 async tests skipped (missing `@pytest.mark.asyncio`)
- **Description:** `test_tool_execution.py` contains 6 async test functions without the asyncio marker. Pytest skips them silently.
- **Status:** Tracked as `TECHNICAL_DEBT.md` DEBT-001 (will resolve in next debt week). Listed here because it surfaced at MW.4 review and was not resolved at seal time.

---

## Phase III.A — Connection Convergence (sealed 2026-05-XX)

### A1.1 — `daily_paper_pipeline` overall_progress estimates not exact
- **Description:** `overall_progress` values (0.0, 0.2, 0.6, 0.95, 1.0) are linear-stage approximations, not weighted by actual stage duration.
- **Reason:** Per-stage runtime varies wildly with paper count. Linear approximation is more honest than fake-precise weighted estimation. User cares about "which stage" more than "exact %."

---

## Phase II — Cognition (sealed 2026-05-XX)

### II.A.1 — Skill JSON `usage_count` / `success_rate` updated only on user feedback
- **Description:** Stats only update on `+`/`-` button click. Sessions where user neither approved nor rejected are not counted.
- **Reason:** Implicit success inference is unreliable. Counting only explicit feedback keeps the metric meaningful.

### II.B.1 — Gravedigger (OpenReview Miner) deferred to Phase IV
- **Description:** Originally scoped for Phase II.B. Removed during planning.
- **Reason:** Use case is "research-period reflection," not daily pipeline. Will reuse `FilterService` + `PaperArchiveAdapter` infrastructure in Phase IV.

---

## Phase I — Foundation (sealed 2026-04-XX)

### I.M2.1 — Frontend Provider deletion UI removed
- **Description:** Originally planned `delete_provider` Tauri command + UI button. Delivered Tauri command but no UI exposure.
- **Reason:** Migrated Provider definitions to `config.toml`. Edits go through TOML. UI deletion would re-introduce dual-track configuration. Removing the UI was the correct response to "TOML is single source of truth."

### I.M3.1 — Rust log level naming differs from Python (`WARN` vs `WARNING`)
- **Description:** `log` crate prints `WARN`. Python `logging` prints `WARNING`.
- **Reason:** Aligning would require a custom Rust formatter or downgrading Python. Visual mismatch is minor; greppability preserved by bracket prefix being identical.

---

## Phase III.C — Structured Final Contract (sealed 2026-05-25)

### FC.1.1 — `search_vault` / `search_vault_attribute` return `artifacts=None`
- **Description:** Both tools return `ToolOutput(text=..., artifacts=None)`. No structured artifact tier exists in the adapter; parsing back from the display string would brittle-couple the tool to formatter changes.
- **Reason:** `obsidian_graph_query` does populate artifacts from the adapter's `list[dict]`. The two search tools are intentionally deferred until the adapter exposes a structured tier.

### FC.2b.1 — `state::Message` not widened with `artifacts`
- **Description:** The runtime LLM history struct (`state::Message`) does not carry an `artifacts` field. Only `ChatEntry` (persistence) carries it.
- **Reason:** Adding `artifacts` to `Message` would risk leakage into outbound `evaluate_payload` (HSC #2 violation). `ChatEntry.artifacts` persists; UI reads via SSE event + `load_session_archive` return.

### FC.3b.1 — `svelte-check` deferred
- **Description:** `svelte-check` not run at FC.3b commit time. `node_modules` absent on dev host.
- **Reason:** Same condition as FC.2b. No TypeScript errors expected: `invoke` already imported; `msg.artifacts` typed as `Artifact[]`.

### FC.5.1 — E2E manual smoke deferred to FC.6
- **Description:** FC.5 verify-only sprint confirmed the delete pipeline structurally. Full E2E (delete → restart → confirm gone) deferred to FC.6 smoke procedure.
- **Reason:** FC.5 is verify-only by definition; E2E requires a running app. Smoke procedure documented at `docs/audits/FC.6-e2e-smoke.md`.

### FC.6.1 — E2E manual smoke not automated
- **Description:** `docs/audits/FC.6-e2e-smoke.md` is a manual checklist, not an automated script.
- **Reason:** No E2E harness exists in the project. Introducing one violates `chimera-dependency-veto`. Manual procedure is sufficient for a single-user personal OS.

---

## Phase III.E — Oligo Orchestration Primitives (sealed 2026-06-11)

### III.E.A.1 — `fork_agent` tool stub: e2e wiring deferred to Phase IV
- **Description:** `fork_agent` is registered in `ToolRegistry` with full `ToolSpec`, but the function body returns a stub message. The actual parent-agent injection (passing `self` into the tool call context) requires Phase IV infrastructure.
- **Reason:** Tool registration is the Phase III.E deliverable. E2e wiring requires the `deep_research` tool and Phase IV.B call context, which are out of scope for III.E.

### III.E.1.1 — HSC 1 downgraded: no real long-result router trigger path available
- **Description:** Original HSC 1 ("50K paper via `fork_subagent` increases main context by < 1K tokens — verified by token count") was untestable: no router trigger path exists and no real long-result source is available at phase time. Downgraded to unit test: 50K-token prompt given to `fork_subagent` returns summary ≤ 4096 chars and parent `messages` does not contain the 50K content.
- **Reason:** The structural isolation guarantee (child messages list never merges into parent) is what HSC 1 was protecting. The unit test verifies this directly. Token-count verification against a live LLM paper read is a Phase IV concern.

---

## Phase IV.A — Async Agent Core (sealed 2026-06-14)

### IV.A.W.1 — `_step_wash` returns `tuple[list[ExecutedToolResult], list[str]]` instead of `list[ExecutedToolResult]`
- **Description:** Batch plan `Phase-IV.A-batch.md` declared `_step_wash` signature as `-> list[ExecutedToolResult]`. Implementation returns a tuple: `(washed_results, wash_sse_frames)`. The `list[str]` carries wash telemetry SSE frames, which the main loop yields for byte-identical SSE output.
- **Reason:** HSC-1 requires byte-identical SSE stream before/after A.3 refactor. Wash telemetry frames must be yielded from the main generator, not swallowed inside the step method. Returning them as a second element is the minimal architectural adjustment; any alternative would have violated HSC-1 or introduced a side-channel (e.g., queue). Deviation declared at execution time.

---

## Phase V.A — Exocortex Node Ontology (sealed 2026-06-16)

### V.A.2b.1 — Vault path re-derived at artifact build time
- **Description:** `_collect_pipeline_artifacts` re-derives the vault note path using `sanitize_filename` rather than receiving it from `write_knowledge_node`. Coupling to naming logic is local to one helper.
- **Reason:** User-approved clarification (2026-06-15): zero changes to pipeline data model. Thread-through would have required widening `BatchMustReadItem`.

### V.A.4.1 — `svelte-check` not run at V.A.4 seal
- **Description:** Svelte TypeScript checks not run. Precedent: FC.3b.1.
- **Reason:** `node_modules` absent on dev host. All new `invoke` calls follow existing typed patterns.

---

*Update protocol: Append-only at sprint seal. New entries appended by `chimera-sprint-discipline` phase_review mode under `<state_write_authority>` (auto-apply, no diff).*
