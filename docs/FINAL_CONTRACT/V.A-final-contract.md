# Phase V.A — Final Contract

**Sealed:** 2026-06-16  
**Batch plan:** `docs/plans/Phase-V.A-batch.md`  
**Audit reference:** `docs/audits/V.A.0.md`  
**Driving frictions resolved:** friction-260611 E1 (structured output), E3 (vault paths)

---

## Sprint Deliverables

### V.A.1 — Schema Authority
- Render smoke: 0 legacy edge names in any `.j2` template. All 5 templates (`knowledge_node`, `deep_read_node`, `thought_node`, `insight_node`, `decision_node`) confirmed spec-aligned.
- `docs/ARCHITECTURE/NODE_ONTOLOGY.md` authored: K/T/I/D frontmatter schema, authoritative edge-name table, immutability/append-only contract, manual vault `templates/` deployment note.
- Vault `templates/` deployment is **user-performed** (copies `Tpl_*.md` from `crucible_core/prompts/obsidian_tpl/` by hand) — agent did not write to vault.

### V.A.2a — Pipeline Result Enrichment
- `_collect_must_read_lines(stats) -> list[str]` extracted in `daily_chimera_service.py`.
- Both `run_daily_pipeline` and `run_daily_pipeline_with_stage_events` summary strings enriched with per-paper `title [score/10]` lines.
- `_render_daily_report` Telegram path untouched — byte-identical output.
- Test: `tests/services/test_daily_chimera_service.py` (3 passing).

### V.A.2b — Structured Artifact Propagation
- `_collect_pipeline_artifacts(stats, inbox_folder) -> list[Artifact]` in `daily_chimera_service.py`: derives vault path via `sanitize_filename` — zero changes to `BatchMustReadItem`, `batch_filter_workflow`, or `write_knowledge_node`.
- `run_daily_pipeline_with_stage_events` returns `ToolOutput(text=summary, artifacts=[...]).model_dump_json()`.
- `check_task_status`: on COMPLETED, tries `ToolOutput.model_validate_json(body)`; parse failure falls back to `[Task Completed] {body}` — backward-compatible for `arxiv_miner`.
- `_run_daily_with_progress`: `-> str` annotation dropped.
- Tests: `test_miner_tools.py` — ToolOutput parse path + legacy fallback (2 new, all passing).

### V.A.3 — Staging Protocol
- `docs/ARCHITECTURE/STAGING_PROTOCOL.md` authored.
- `StagingService(staging_dir, vault_root)` at `crucible_core/src/crucible/services/staging_service.py`:
  - `create_staging_node(type, title, body, edges?) -> Path` — writes PENDING_REVIEW candidate to `docs/staging/`.
  - `promote_node(path) -> vault_path` — sets `status: active` (all types), writes to vault subdir, removes staging file.
  - `reject_node(path)` — deletes staging file.
- Tests: `tests/services/test_staging_service.py` (5 passing).

### V.A.4 — Astrocyte One-Click Node Creation
- **Backend:** `crucible_core/src/oligo/api/staging_routes.py` (4 endpoints: create/list/promote/reject), registered in `server.py`.
- **Rust:** `no_traversal` path guard + 4 Tauri commands (`list_staging_candidates`, `create_staging_node_cmd`, `promote_staging_node`, `reject_staging_node`) in `lib.rs`. Cargo check: clean.
- **Svelte:** `StagingPanel.svelte` (5s polling, promote/reject buttons). `createThoughtFromMessage` function + N button in BB message action row. `<StagingPanel />` mounted in `hud-output`.
- No LLM involvement in staging operations (UI→Tauri→HTTP→StagingService).

### V.A.5 — vault_query Tool
- `vault_query(type?, status?, linked_to?)` at `crucible_core/src/oligo/tools/vault_query.py`.
- Uses ripgrep subprocess to find candidate files, then Python YAML frontmatter parse + in-process filter.
- Registered in `ToolRegistry` (`concurrency_safe=True, long_running=False`).
- Returns `title + path + type/status excerpt` per match.
- Tests: `tests/oligo/test_vault_query.py` (7 passing: parse unit, filter logic, no-filter guard, latency smoke).

---

## Accepted Partials

### V.A.2b.1 — Vault path re-derived at artifact build time
- **Description:** `_collect_pipeline_artifacts` re-derives the vault note path using `sanitize_filename` rather than receiving it from `write_knowledge_node`. Path accuracy depends on naming logic staying in sync with the writer.
- **Reason:** User-approved clarification (2026-06-15): "do NOT thread vault_path through the pipeline. Re-derive it." Zero changes to pipeline data model. Coupling is local to one helper function.

### V.A.4.1 — `svelte-check` not run
- **Description:** Svelte TypeScript checks not run at seal time.
- **Reason:** `node_modules` absent on dev host. Precedent: FC.3b.1. No TypeScript errors expected: `invoke` already imported; all new calls follow existing patterns.

---

## Hard Sealing Conditions — Status

| # | Condition | Verification method | Status |
|---|---|---|---|
| 1 | K/T/I/D templates exist as `.md` with typed frontmatter | `NODE_ONTOLOGY.md` authored; `Tpl_*.md` files staged in repo; vault copy **user-performed** | ⬜ Pending user vault copy |
| 2 | `daily_paper_pipeline` surfaces ≥1 Artifact per paper; BB reply lists real titles | V.A.2a+2b unit tests pass; E2E: run pipeline, call `check_task_status`, verify `ToolOutput` | ⬜ Pending E2E (V.A.6 smoke) |
| 3 | Staging: candidate T Node → PENDING_REVIEW → promote → vault / reject → delete | V.A.3 unit tests pass; E2E: N button in Astrocyte → staging panel flow | ⬜ Pending E2E (V.A.6 smoke) |
| 4 | `vault_query(type="knowledge")` returns results in<2s | V.A.5 unit tests pass (mocked); E2E: live query against vault | ⬜ Pending E2E (V.A.6 smoke) |
| 5 | Node ratio K:T:I:D ≈ 4:8:2:1 after Use Week | Post-seal usage review | ⬜ Deferred (post-seal Use Week) |

---

## Out of Scope (carried forward → Phase VI+)

- Embedding / vector search / PPR / graph random walk
- Automatic node creation without human review
- Inbox note migration (360/400 vault notes untyped — vault_query correctly ignores them)
- Gravedigger / OpenReview miner
- Obsidian plugin development
- `search_vault` precision improvement (DEBT-003 — deferred to Phase V.E/VI)
