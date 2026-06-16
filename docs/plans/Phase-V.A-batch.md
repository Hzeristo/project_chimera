# Batch Plan: Phase V.A — Exocortex Node Ontology & Research Production Line

**Output location:** `docs/plans/Phase-V.A-batch.md`
**Audit reference:** `docs/audits/V.A.0.md` (date: 2026-06-14)
**Phase doc:** `docs/phases/phase-V.A.md` (sparse manifest — distinct from this batch plan)
**Driving frictions:** `friction-260611` E1 residual (structured output), E3 residual (vault-link paths). Friction labels as declared in `phase-V.A.md:5-11`; source file `docs/logs/friction-260611.md` uses entry numbering.

This document is a single unit. User approves the whole sequence or rejects
the whole sequence. After approval, hand off to `chimera-code-taste`
batch_execution mode.

**STATUS: APPROVED (2026-06-15) — execution NOT yet authorized; stop before any code change.**

**User clarifications at approval (override conflicting text below):**
1. V.A.1 — vault `templates/` deployment is **MANUAL** (user copies by hand).
   Sprint scope = render confirmation + `NODE_ONTOLOGY.md` ONLY.
   The agent does NOT write to the vault `templates/` directory.
2. V.A.2b — `Artifact.path` points to the **VAULT inbox note**
   (`inbox/Must_Read/` or `inbox/Skim/`), NOT the filtered MD under `papers/`.
   `VaultNoteWriter.write_knowledge_node` writes + returns this path before the
   pipeline returns, so the vault path is resolved and safe at artifact-build time.

**Pre-sprint work already landed (NOT re-planned here):**
- K/T/I/D Jinja2 render templates authored/verified through `PromptManager`:
  `crucible_core/prompts/obsidian_tpl/{knowledge,deep_read,thought,insight,decision}_node.j2`
  (+ preserved `deep_read_survey_node.j2`). Confirmed present via glob.
- `Tpl_{knowledge,thought,insight,decision}.md` Templater templates authored
  (untracked in `crucible_core/prompts/obsidian_tpl/`, per `git status`).
- Vault frontmatter migration executed (`scripts/migrate_vault_frontmatter.py --apply`):
  4 typed nodes migrated to spec edge names; 0 old names remain; backups at `vault/.migration_backup/`.
- **Audit Cross-finding §1** (edge-name mismatch) → RESOLVED by migration. Spec names win.
- **Audit Cross-finding §2** (missing `obsidian_tpl` path) → FALSE ALARM: templates live at
  `crucible_core/prompts/obsidian_tpl/`; audit glob (`crucible_core/src/**/*.j2`) missed the
  top-level `prompts/` dir. Not a real gap. V.A.1 scope reduced accordingly.

---

## Sprint Sequence

```
V.A.0 (audit, done) → V.A.1 (schema authority: deploy + confirm + doc) ┐
                                                                       │
        ┌──────────────────────────────────────────────────────────────┤
        │                                                              │
   V.A.2 (structured pipeline output) ── independent ──────────────────┤
        │                                                              ├→ V.A.6 (seal)
   V.A.3 (staging protocol, backend) → V.A.4 (Astrocyte cmds + panel) ──┤
        │                                                              │
   V.A.5 (vault_query tool, needs V.A.1 schema) ──────────────────────┘
```

Dependencies (from `phase-V.A.md:36-40`): V.A.1 precedes V.A.2/V.A.3/V.A.5
(authoritative schema is the substrate). V.A.2 independent of V.A.3–V.A.5.
V.A.4 depends on V.A.3 (staging protocol). V.A.5 depends on V.A.1 (frontmatter schema).
V.A.6 after all. Predecessor assumptions stated per sprint below.

---

## Sprint V.A.1: Schema Authority — deploy templates, confirm writer, document ontology

**Anticipatory justification:** No friction logged on K/T/I/D creation yet; the
phase doc (`phase-V.A.md:10-12`) is the explicit user override ("Phase IV.A's async
core makes it possible. This phase builds the consumption layer"). Scope is now
*reduced* — authoring + migration already landed pre-sprint.

**Predecessor assumptions:**
- Pre-sprint claim holds: all 6 `*_node.j2` render cleanly through `PromptManager`
  and migration left 0 old edge names. Re-plan trigger if a render or grep check fails.

**Risk level:** 🟢 LOW
- Read-only verification + doc authoring only. NO vault writes — user deploys `Tpl_*.md`
  by hand.

### 目标
Confirm `VaultNoteWriter` renders the spec-aligned `.j2` templates to valid frontmatter,
and document the authoritative K/T/I/D schema + edge names as the canonical reference.
Vault `templates/` deployment is **user-performed manually** outside this sprint.

### 设计要点(audit-derived)
- Spec edge names (`derives_from`, `supersedes`, `contradicts`, `dead_ends`,
  `synthesizes`, `depends_on`) are authoritative — migration already enforced them;
  the legacy Tpl_ names (`derived_from`, `promoted_to_insight`, …) are superseded —
  audit Cross-finding §1, `phase-V.A.md:86-99`.
- Vault `templates/` still holds OLD files using legacy names — audit Q3,
  `vault://templates/Tpl_thought.md:8-14`. The user will copy the new `Tpl_*.md` files
  from `crucible_core/prompts/obsidian_tpl/` into the vault by hand; the agent
  **does not touch** `vault://templates/`.
- `VaultNoteWriter.write_knowledge_node` / `write_deep_read_node` render via
  `PromptManager` from `obsidian_tpl/` — `vault_note_writer.py:31-45,47-74`. Confirm
  the already-staged `.j2` templates produce valid YAML frontmatter at the writer level.
- Decision template is greenfield (D folder empty, no `Tpl_decision` legacy) — audit Q10.

### 任务范围
1. Render smoke: verify `write_knowledge_node` + `write_deep_read_node` produce frontmatter
   with spec edge names against the staged `.j2` templates — no source change expected,
   grep/render-check only (`vault_note_writer.py:31-45,47-74`) — audit Q5/Q6.
2. Author `docs/ARCHITECTURE/NODE_ONTOLOGY.md`: K/T/I/D frontmatter schema,
   authoritative edge-name table, immutability/append-only contract, note on manual
   vault `templates/` deployment procedure — `phase-V.A.md:85-112`.

### 验收
- `write_knowledge_node` render smoke produces frontmatter whose edge keys match spec set;
  `Select-String` for legacy names returns 0 hits in rendered output.
- `NODE_ONTOLOGY.md` exists with authoritative edge-name table; no spec divergence remains.
- *(Out of sprint scope — user action)* Vault `templates/` populated with new `Tpl_*.md`
  before V.A.6 E2E smoke.

### 红线
- ❌ NO agent writes to vault `templates/` — deployment is manual (user clarification 2026-06-15).
- ❌ NO new edge-name set — spec names decided by migration; do not re-litigate.
- ❌ NO Obsidian plugin dependency — Templater templates are plain `.md` (phase-wide).
- ❌ 不进行机会主义重构 — do not refactor `VaultNoteWriter`.

### 输出位置
- 代码: none (render smoke is read-only verification)
- 文档: `docs/ARCHITECTURE/NODE_ONTOLOGY.md` (new)

## Sprint V.A.2a: Pipeline result enrichment — real paper data in task result text

**Friction reference:** `friction-260611` E1 residual (`phase-V.A.md:6-7`) — agent
synthesizes Final from terse summary string, not real per-paper data. ACTIVE.

**Predecessor assumptions:**
- None — independent of V.A.1/V.A.3/V.A.5. Touches only the service return text.

**Risk level:** 🟡 MED
- Single-file change, ~15 lines, has a clear before/after assertion target.

### 目标
Enrich the daily-pipeline task-completion text so the agent's Final synthesis lists
real paper titles + scores, replacing the aggregate-counts-only summary string.

### 设计要点(audit-derived)
- Per-paper data already exists in-memory at pipeline end: `stats.must_read_items`
  with `.score / .id / .filename / .short_moniker / .title / .novelty` — confirmed
  `daily_chimera_service.py:267,270-308` (today consumed ONLY by `_render_daily_report`
  for Telegram; never surfaced to the task result).
- The task result is the agent's synthesis substrate: `run_daily_pipeline_with_stage_events`
  returns a counts-only string at `daily_chimera_service.py:249-255`; that string becomes
  `Task.result` → `TaskEvent.message` (`task_service.py:197,209`) → IV.A `await_completio
  Thought for 1m 13s, searched for 2 patterns, read 2 files

Now I have the full picture. The critical V.A.2 structural finding:

TaskService.run_task accepts Awaitable[str] and Task.result: str | None — no artifact propagation path exists through the task bus. The architecture requires a deliberate split: V.A.2a enriches the task result text (direct friction fix), V.A.2b introduces ToolOutput JSON serialization through check_task_status (artifact chip path). Neither requires changing TaskService's schema.

Writing the remaining sprints now.

  Read 1 file

  Thought for 4s

✻ Crunched for 4m 7s

❯ /model
  ⎿  Set model to Opus 4.8 and saved as your default for new sessions

❯ perform actual write to the console, i will manually copy and paste into the plan.

Understood — I'll output the remaining batch-plan sections (V.A.2a through V.A.6 + phase-wide red lines + sealing conditions) to the console for you to paste in, instead of writing to the file.

---
---

## Sprint V.A.2a: Pipeline result enrichment — real paper data in task result text

**Friction reference:** `friction-260611` E1 residual (`phase-V.A.md:6-7`) — agent
synthesizes Final from terse summary string, not real per-paper data. ACTIVE.

**Predecessor assumptions:**
- None — independent of V.A.1/V.A.3/V.A.5. Touches only the service return text.

**Risk level:** 🟡 MED
- Single-file change, ~15 lines, has a clear before/after assertion target.

### 目标
Enrich the daily-pipeline task-completion text so the agent's Final synthesis lists
real paper titles + scores, replacing the aggregate-counts-only summary string.

### 设计要点(audit-derived)
- Per-paper data already exists in-memory at pipeline end: `stats.must_read_items`
  with `.score / .id / .filename / .short_moniker / .title / .novelty` — confirmed
  `daily_chimera_service.py:267,270-308` (today consumed ONLY by `_render_daily_report`
  for Telegram; never surfaced to the task result).
- The task result is the agent's synthesis substrate: `run_daily_pipeline_with_stage_events`
  returns a counts-only string at `daily_chimera_service.py:249-255`; that string becomes
  `Task.result` → `TaskEvent.message` (`task_service.py:197,209`) → IV.A `await_completion`
  → Final. Enriching THIS string is the minimal friction fix.
- `_render_daily_report`'s title-building logic (`:276-281`: `short_moniker` → `paper_id+moniker`
  → `legacy_title` → `paper_id`) is the canonical title derivation — reuse it, do not re-

### 任务范围
1. Extract a shared `_collect_must_read_lines(stats) -> list[str]` (or equivalent) that
   reuses the existing title-derivation, so both Telegram render and task summary draw from
   one source (`daily_chimera_service.py`, ~10 lines) — audit Q11, Cross-finding §3.
2. Append the per-paper title+score lines to the summary returned by
   `run_daily_pipeline_with_stage_events` (and, for parity, sync `run_daily_pipeline`)
   (`daily_chimera_service.py:249-255,152-158`, ~5 lines) — audit Q11.

### 验收
- A completed pipeline with ≥1 must_read paper produces a task result string containing
  the actual paper titles, not just `must_read={n}` — verifiable via unit test asserting
  title substring in the returned summary.
- `_render_daily_report` Telegram output byte-identical to before (shared helper must not
  alter the Telegram path) — verifiable via existing Telegram render test / snapshot.

### 红线
- ❌ NO modification of existing vault notes (phase-wide).
- ❌ NO change to TaskService schema in this sprint (deferred consideration → V.A.2b).
- ❌ NO change to Telegram HTML output — shared helper is render-neutral.
- ❌ 不进行机会主义重构 — do not refactor the pipeline staging/progress logic.

### 输出位置
- 代码: `crucible_core/src/crucible/services/daily_chimera_service.py`
- 测试: `tests/crucible/services/test_daily_chimera_service.py` (summary-content assertion)
- 文档: 推迟至 V.A.6 统一更新

---

## Sprint V.A.2b: Structured artifact propagation — ToolOutput through check_task_status

**Friction reference:** `friction-260611` E3 residual (`phase-V.A.md:8-9`) — tool results
lack file paths; frontend can't link to vault/MD notes from BB replies. ACTIVE.

**Predecessor assumptions:**
- V.A.2a landed: `stats.must_read_items` is already surfaced via a shared collector helper,
  so V.A.2b reuses it for `Artifact` construction. Re-plan trigger if V.A.2a did not extract
  a reusable per-paper accessor.

**Risk level:** 🔴 HIGH
- Touches the long-running tool result contract across 2 files; introduces a serialized
  `ToolOutput` round-trip through a `str`-typed task bus. No existing test harness for the
  completed-task artifact path. **Requires explicit per-sprint approval before execution.**

### 目标
Make a completed daily pipeline surface a `ToolOutput` with one `Artifact` per must_read
paper through `check_task_status`, so BB replies carry vault/MD-file chips (FC.2 chip pat

### 设计要点(audit-derived)
- **Structural blocker confirmed (NOT in audit — found during planning):** `TaskService`
  has NO artifact channel. `Task.result: str | None` (`task_service.py:44`); `run_task`
  accepts `Awaitable[str]` (`task_service.py:353`); `TaskEvent` carries no artifacts. The
  pipeline's `ToolOutput` cannot flow through the task bus as a typed object.
- **Chosen path — serialize, don't widen TaskService:** store `ToolOutput.model_dump_json()`
  as `Task.result` (still a `str`; TaskService stays artifact-agnostic). `check_task_stat
mpleted result as `ToolOutput`; on success returns the `ToolOutput`
  (text+artifacts), on parse failure returns the raw string as before — backward-compatible
  for arxiv_miner and any other task type. This avoids modifying core `Task`/`TaskEvent`
  schema (which would ripple into the IV.A await path and the Astrocyte SSE contract).
- `Artifact` shape ready: `Artifact(kind, path, metadata)` — `oligo/core/schemas.py:132-148`;
  `ToolOutput(text, artifacts)` — `:151-163`. Each artifact: `kind="vault_note"`,
  `path=` **vault inbox note path returned by `write_knowledge_node`** —
  `inbox_folder / verdict / {fancy_basename}.md` (`vault_note_writer.py:40-44`); writer
  completes before pipeline returns so path is resolved and stable at artifact-build time
  (user clarification 2026-06-15). `metadata={arxiv_id, verdict, score}` — `phase-V.A.md:114-119`.
- `check_task_status` is the surfacing point because the tool launch returns before
  completion (`miner_tools.py:106-119` returns a task-started string); the `ExecutedToolResult`
  for the launch call is immutable and predates completion. Artifacts must ride the
  status-poll/await-completion result, not the launch return — audit Q13, Cross-ref.

### 任务范围
1. Build `ToolOutput` at pipeline end: `_collect_pipeline_artifacts(stats) -> list[Artifact]`
   constructs one `Artifact` per must_read item using the vault note path written by
   `write_knowledge_node` (path resolved as `inbox_folder / verdict / {fancy_basename}.md`
   per `vault_note_writer.py:40-44`); wrap V.A.2a summary as
   `ToolOutput(text=summary, artifacts=...)`; return `model_dump_json()` from
   `run_daily_pipeline_with_stage_events` (`daily_chimera_service.py`, ~20 lines) —
   audit Q11/Q12. **Execution note:** verify how `batch_filter_workflow` surfaces the
   written vault paths back to the service layer before coding the collector.
2. `_run_daily_with_progress` passes the JSON through unchanged (already `return await`,
   drop the `-> str` annotation) (`miner_tools.py:49-67`, ~2 lines).
3. `check_task_status`: on `COMPLETED`, try `ToolOutput.model_validate_json(body)`; succe
   → return `ToolOutput`; `ValidationError`/JSON error → return `[Task Completed] {body}`
   string as today (`miner_tools.py:143-145`, ~10 lines) — audit Q13.

### 验收
- Completed pipeline → `check_task_status` returns a `ToolOutput` whose `artifacts` list
  one entry per must_read paper, each with a resolvable MD `path` + `{arxiv_id, verdict,
  score}` metadata — verifiable via unit test on a stubbed completed task.
- A completed `arxiv_miner` (plain-string result) → `check_task_status` still returns the
  legacy `[Task Completed] …` string (no regression) — verifiable via unit test.
- Phase sealing #2: BB reply after pipeline lists real titles — verifiable at V.A.6 E2E.

### 红线
- ❌ NO widening of `Task` / `TaskEvent` schema — serialization path only (sprint-specific).
- ❌ NO artifact path that re-parses the human display string — build from `stats`, not text
   (audit Cross-finding §3 explicitly warns against string re-parse).
- ❌ NO modification of existing vault notes (phase-wide).
- ❌ 不进行机会主义重构 — do not refactor TaskService or the IV.A await path.

### 输出位置
- 代码: `crucible_core/src/crucible/services/daily_chimera_service.py`,
  `crucible_core/src/oligo/tools/miner_tools.py`
- 测试: `tests/oligo/tools/test_miner_tools.py` (ToolOutput parse + legacy fallback)
- 文档: 推迟至 V.A.6 统一更新

---

## Sprint V.A.3: Staging protocol — docs/staging/ lifecycle + StagingService (backend)

**Anticipatory justification:** No friction logged on node creation; phase doc
(`phase-V.A.md:121-125`) is the explicit user override — staging is the human-gated
mechanism (L2 philosophy) that the consumption layer requires before V.A.4 can wire UI.

**Predecessor assumptions:**
- V.A.1 fixed the authoritative T/I/D frontmatter schema + edge names. Staging candidate
  files must be born with spec frontmatter. Re-plan trigger if V.A.1 left schema ambiguous.

**Risk level:** 🔴 HIGH
- Greenfield service + lifecycle, >3 files / >30 lines, no prior convention to anchor.
  **Requires explicit per-sprint approval before execution.**

### 目标
Build the `docs/staging/` PENDING_REVIEW → PROMOTED / REJECTED protocol and a Python
`StagingService` (create / promote / reject) operating on candidate `.md` node files.

### 设计要点(audit-derived)
- Fully greenfield: `docs/staging/` does not exist; no PENDING_REVIEW lifecycle, no promo
  reject mechanism, no candidate format anywhere in repo — audit Q14, Cross-ref (`grep
  PENDING_REVIEW|staging → 0 hits`).
- Candidate node = a `.md` file with spec frontmatter (`type`, `status: PENDING_REVIEW`,
  `created_at`, typed-edge fields) — V.A.1 schema is the substrate; default candidate type
  is `thought` (`phase-V.A.md:31,72-74` — BB conversations surface candidate T nodes).
- **Promote = vault write.** Promotion moves the file into the vault proper (T → vault
  `Thoughts/`, I → `Insight/`, D → `Decision/`), flipping `status` to the type's active
  value. This is the ONE sanctioned vault-write path in V.A and is human-triggered — it does
  NOT violate the "no auto node creation" red line (`phase-V.A.md:46`). Reject = delete the
  staging file (`phase-V.A.md:73-74`).
- vault_root resolution single-sourced from `SystemConfig.vault_root` (`config.py:129`,
  validator `:392`) — staging must not hardcode the destination — audit Cross-ref.
- Astrocyte exposure is V.A.4; this sprint is backend protocol + service + tests only,
  callable headless.

### 任务范围
1. Author staging protocol doc: directory layout, candidate frontmatter contract, lifecycle
   state machine, promote-destination-by-type mapping (`docs/staging/README.md` or
   `docs/ARCHITECTURE/STAGING_PROTOCOL.md`, new) — audit Q14.
2. `StagingService.create_staging_node(type, title, body, edges) -> path` — writes a
   PENDING_REVIEW candidate to `docs/staging/` (new module under
   `crucible_core/src/crucible/services/`, ~30 lines) — audit Q14.
3. `StagingService.promote_node(staging_path) -> vault_path` — resolves destination by ty
   flips status, writes to vault, removes staging file; `reject_node(staging_path)` — deletes
   candidate (~30 lines) — `phase-V.A.md:73-74`.

### 验收
- `create_staging_node(type="thought", …)` produces a `docs/staging/*.md` with
  `status: PENDING_REVIEW` and spec edge fields — verifiable via unit test reading frontmatter.
- `promote_node(path)` writes the node under the correct vault subfolder with active status
  and removes the staging file; `reject_node(path)` removes the staging file — verifiable

### 红线
- ❌ NO automatic promotion — create lands at PENDING_REVIEW only; promote is explicit
  (phase-wide, `phase-V.A.md:46`).
- ❌ NO modification of existing vault notes — promote writes NEW files only (phase-wide).
- ❌ NO staging in the vault — candidates live in repo `docs/staging/`, not in vault_root
  (sprint-specific).
- ❌ 不进行机会主义重构.

### 输出位置
- 代码: `crucible_core/src/crucible/services/staging_service.py` (new)
- 测试: `tests/crucible/services/test_staging_service.py`
- 文档: `docs/ARCHITECTURE/STAGING_PROTOCOL.md` (this sprint — protocol must exist to anchor)

---

## Sprint V.A.4: Astrocyte one-click node creation — Tauri commands + staging panel

**Friction reference:** `friction-260611` E3 residual (`phase-V.A.md:8-9`) — frontend
can't act on vault notes from BB. Node creation from BB conversation is the consumption-layer
payoff. ACTIVE (mechanism-level).

**Predecessor assumptions:**
- V.A.3 landed: `StagingService` create/promote/reject exist and are callable from the
  backend HTTP surface. Re-plan trigger if V.A.3 service signatures differ from assumed.

**Risk level:** 🔴 HIGH
- Cross-stack: Python HTTP endpoints + Rust Tauri commands + Svelte panel, >3 files.
  **Requires explicit per-sprint approval before execution.**

### 设计要点(audit-derived)
- Only one vault-facing Tauri command exists today: `open_vault_note` (`lib.rs:1149-1188`),
  which validates via `vault_contains_path` (`lib.rs:1117-1147`, traversal + canonicalize
  guard) then opens an `obsidian://` URI. No `create_note`/`write_note`/`staging` commands —
  audit Q13, Cross-ref (`grep create_note|write_note|staging → 0 hits`).
- File writes belong to the Python backend (Tauri opens URIs, it does not own vault writes
  in the III.C pattern). V.A.4 commands must call backend HTTP endpoints wrapping V.A.3's
  `StagingService`, mirroring how Astrocyte triggers other CLI workflows — `phase-V.A.md:32`.
- `vault_root` read from `AstrocyteState` (`config.rs:30`, `lib.rs:1158-1166`) is the single
  source for any path display — audit Cross-ref.
- "Create candidate Thought Node from hovered BB message" → `create_staging_node`; staging
  panel lists PENDING_REVIEW candidates with promote/reject — `phase-V.A.md:32`.

### 目标
Wire hover-BB-message → create candidate Thought node in staging, and a staging panel wit
promote/reject, via three new Tauri commands backed by V.A.3's StagingService.

### 任务范围
1. Backend HTTP endpoints wrapping `StagingService` create/promote/reject (FastAPI route
   module in `crucible_core`, ~30 lines) — audit Q13.
2. Three Tauri commands `create_staging_node` / `promote_node` / `reject_node` alongside
   `open_vault_note`, reusing the `vault_contains_path` guard for any path argument
   (`astrocyte/src-tauri/src/lib.rs`, ~60 lines) — audit Q13, Cross-ref.
3. Svelte staging panel: hover-message → create action; PENDING_REVIEW list with
   promote/reject buttons (`astrocyte/src/…`, component + invoke wiring) — `phase-V.A.md:32`.

### 验收
- Hovering a BB message and triggering "create node" yields a PENDING_REVIEW candidate in
  `docs/staging/` — verifiable via manual flow (sealing #3).
- Staging panel lists candidates; Promote moves to vault + removes from panel; Reject deletes
  — verifiable via manual flow (sealing #3).
- Path arguments crossing the Tauri boundary pass `vault_contains_path` (no traversal) —
  verifiable via Rust unit test on the guard with `..` input.

### 红线
- ❌ NO Tauri-side file write that bypasses the backend StagingService (sprint-specific).
- ❌ NO automatic promotion — UI promote is an explicit user click (phase-wide).
- ❌ NO modification of existing vault notes (phase-wide).
- ❌ 不进行机会主义重构 — no refactor of `open_vault_note` or `AstrocyteState`.

### 输出位置
- 代码: `crucible_core/src/…` (HTTP routes), `astrocyte/src-tauri/src/lib.rs`,
  `astrocyte/src/…` (Svelte panel)
- 测试: Rust guard unit test; manual flow checklist for the cross-stack path (deferred to V.A.6)
- 文档: 推迟至 V.A.6 统一更新

---

## Sprint V.A.5: vault_query tool — frontmatter-indexed ripgrep query

**Anticipatory justification:** No friction logged yet (DEBT-003 notes `search_vault`
imprecision but defers to Exocortex); phase doc (`phase-V.A.md:33,127-132`) makes
frontmatter query the substrate for daily research retrieval — "grep, not search."

**Predecessor assumptions:**
- V.A.1 fixed authoritative edge names (`derives_from`, `supersedes`, `contradicts`,
  `dead_ends`, `synthesizes`, `depends_on`) and migration left 0 old names in the vault.
  vault_query greps the SPEC names. Re-plan trigger if V.A.1 chose different names or
  migration coverage was incomplete (audit Q3 reported 0 hits post-migration).

**Risk level:** 🔴 HIGH
- New tool + ripgrep subprocess + frontmatter parse + ToolRegistry registration, >30 lines.
  **Requires explicit per-sprint approval before execution.**

### 目标
Add a `vault_query(type?, status?, linked_to?)` tool that ripgreps vault frontmatter and
returns matching notes' title + path + frontmatter excerpt in <2s on ~1000 notes.

### 设计要点(audit-derived)
- Dumb-but-fast by mandate: ripgrep over frontmatter YAML, no DB, no index file, no daemo
  no ranking, no embedding — `phase-V.A.md:50-51,127-132`.
- Query SPEC edge names — migration made these authoritative; audit Q3 confirms 0 legacy
  names remain (`grep derives_from|supersedes|contradicts|dead_ends → 0` pre-migration, now
  the only set in use). `linked_to` filters by typed-edge frontmatter containing the target.
- vault_root from `SystemConfig.vault_root` (`config.py:129`) — no hardcoded path.
- Registered as a Tool (atomic capability, Four-Layer model), concurrency-safe, NOT
  long_running — distinct from the daily pipeline. Follows the `ToolSpec` registration
  pattern used for existing tools (`oligo/tools/registry.py`).
- **Coverage caveat to surface (audit Cross-finding §4):** 360/400 vault notes are untyped
  inbox triage dumps; they will never match `type=*` queries. vault_query is correct to
  ignore them; the plan must NOT scope inbox migration (`phase-V.A.md` out-of-scope).

### 任务范围
1. `vault_query` tool fn: build ripgrep args from `type/status/linked_to`, run subprocess
   over vault_root, parse matched files' frontmatter, return title+path+excerpt (new tool
   module under `crucible_core/src/oligo/tools/`, ~40 lines) — audit Q3, `phase-V.A.md:33`.
2. Register in `ToolRegistry` with `ToolSpec` (concurrency_safe=True, long_running=False,
   args_schema, examples) (`oligo/tools/registry.py`, ~15 lines) — Cross-ref.

### 验收
- `vault_query(type="knowledge")` returns the deep-read/knowledge notes with title + path +
  frontmatter excerpt — verifiable via live query against the real vault.
- `vault_query(type="thought", status="dead_end")` filters correctly on the migrated spec
  fields — verifiable via live query (sealing #4).
- Latency <2s on current vault (400 notes; target spec is 1000) — verifiable via timed run
  (sealing #4).

### 红线
- ❌ NO SQLite / index file / daemon / embedding / vector search (phase-wide,
  `phase-V.A.md:44,50-51`).
- ❌ NO ranking or relevance scoring — raw matches only (sprint-specific).
- ❌ NO migration of inbox notes to make them queryable (out-of-scope, Cross-finding §4).
- ❌ 不进行机会主义重构 — do not touch `search_vault` / `search_vault_attribute`.

### 输出位置
- 代码: `crucible_core/src/oligo/tools/vault_query.py` (new), `oligo/tools/registry.py`
- 测试: `tests/oligo/tools/test_vault_query.py` (filter construction + parse + latency smoke)
- 文档: 推迟至 V.A.6 统一更新

---

## Sprint V.A.6: Seal — FINAL_CONTRACT doc + E2E smoke + phase review

**Anticipatory justification:** Phase doc seal sprint (`phase-V.A.md:34`). Required to
verify Hard Sealing Conditions and produce the phase contract.

**Predecessor assumptions:**
- V.A.1–V.A.5 all landed. Re-plan trigger if any sprint is unsealed or carries an unresolved
  red-line halt.

**Risk level:** 🟡 MED
- Doc + manual E2E + state-file updates; no new production code. (State-file writes to
  ROADMAP/ACCEPTED_PARTIALS/TECHNICAL_DEBT happen under phase_review `<state_write_author

### 目标
Produce the V.A FINAL_CONTRACT doc, run the E2E smoke verifying all five Hard Sealing
Conditions, and run phase_review to seal Phase V.A.

### 设计要点(audit-derived)
- Five Hard Sealing Conditions carried from `phase-V.A.md:61-81` (templates, structured
  output, staging flow, vault_query latency, node-ratio — ratio verified at review, not
  sprint level).
- E2E smoke is a manual checklist, not an automated harness (precedent: FC.6 smoke,
  ACCEPTED_PARTIALS FC.6.1 — introducing an E2E harness violates chimera-dependency-veto).
- Sealing #2 (BB reply lists real titles) exercises V.A.2a+V.A.2b end-to-end; #3 exercises
  V.A.3+V.A.4; #4 exercises V.A.5.

### 任务范围
1. Author `docs/FINAL_CONTRACT/V.A-final-contract.md` (or phase-conventional location):
   what each sprint delivered, accepted partials, sealing-condition verification results.
2. Author + execute `docs/audits/V.A.6-e2e-smoke.md` manual checklist covering sealing #1–#4.
3. Run phase_review: propose ROADMAP / ACCEPTED_PARTIALS / TECHNICAL_DEBT diffs; seal on

### 验收
- All five sealing conditions recorded PASS (or documented accepted-partial) in the contract.
- E2E smoke checklist executed with results logged.
- ROADMAP V.A row moves to Sealed with commit refs (under phase_review authority).

### 红线
- ❌ NO new production code in the seal sprint (sprint-specific).
- ❌ NO automated E2E harness (chimera-dependency-veto; ACCEPTED_PARTIALS FC.6.1 precedent).
- ❌ NO modification of existing vault notes (phase-wide).
- ❌ 不进行机会主义重构.

### 输出位置
- 文档: `docs/FINAL_CONTRACT/V.A-final-contract.md`, `docs/audits/V.A.6-e2e-smoke.md`
- 状态: ROADMAP.md / ACCEPTED_PARTIALS.md / TECHNICAL_DEBT.md (phase_review auto-apply)

---

## Phase-wide Red Lines

These apply across ALL sprints in this batch. Violation in any sprint halts the batch:

- ❌ NO embedding / vector DB / semantic search / PPR / graph random walk (Phase VI) —
  `phase-V.A.md:44-45`.
- ❌ NO automatic node creation without human review — staging + PENDING_REVIEW is the
V.A.md:46`.
- ❌ NO modification of existing vault notes without explicit in-conversation user consent;
  new nodes only (promote writes NEW files); existing notes immutable — `phase-V.A.md:47-49`.
- ❌ vault_query uses ripgrep + frontmatter parse ONLY — no SQLite, no index file, no daemon
  — `phase-V.A.md:50-51`.
- ❌ K Nodes IMMUTABLE; T Nodes APPEND-ONLY — `phase-V.A.md:52-55`.
- ❌ NO Obsidian plugin dependencies — file-level read/write `.md` via Python only —
  `phase-V.A.md:56-57`.
- ❌ ARA alignment is STRUCTURAL (template fields → ARA layers), not platform-level —
  `phase-V.A.md:58-59`.
- ❌ Do NOT generalize for multiple users; do NOT add SaaS-shaped abstractions
  (chimera-core-philosophy, CLAUDE.md hard rule).
- ❌ 不进行机会主义重构 in any sprint.

---

## Hard Sealing Conditions (carried from phase doc, `phase-V.A.md:61-81`)

These MUST pass at phase_review for sealing:

1. **(Templates)** K/T/I/D templates exist as `.md` with typed frontmatter, Templater-
   compatible; each has type, status, created_at, typed-edge fields — verified by V.A.1 vault
   `templates/` listing + `NODE_ONTOLOGY.md`.
2. **(Structured output)** `daily_paper_pipeline` surfaces ≥1 Artifact per processed pape
   (title, arxiv_id, verdict, vault/MD path); BB reply lists real titles, not terse-summary
   keywords — verified by V.A.2a+V.A.2b + V.A.6 E2E.
3. **(Staging)** A BB-conversation candidate T node lands in `docs/staging/` as
   PENDING_REVIEW; promote moves it to the vault; reject deletes it — verified by V.A.3+V.A.4
   manual flow.
4. **(vault_query)** "All K Nodes about memory" returns title+path+frontmatter excerpt in
   <2s on current vault size — verified by V.A.5 live query.
5. **(Node ratio)** After one Use Week, vault trends toward K:T:I:D ≈ 4:8:2:1 — verified at
   Phase V.A review (post-seal usage), NOT at sprint level.

---

## Approval

User approves whole sequence or rejects whole sequence.

Upon approval, hand off to `chimera-code-taste` with:
> "Execute batch for Phase V.A per `docs/plans/Phase-V.A-batch.md`."

Note: 🔴 HIGH sprints (V.A.2b, V.A.3, V.A.4, V.A.5) require explicit per-sprint approval
before execution even after batch approval; 🟡 MED sprints (V.A.1, V.A.2a, V.A.6) self-execute
per-sprint and halt the batch on any red-line violation.

---

*Generated by chimera-sprint-discipline batch_planning mode.*
