# Phase Audit: Phase V.A — Exocortex Node Ontology & Research Production Line

**Scope:** Read-only audit prerequisite for batch_planning of Phase V.A.
**Output location:** `docs/audits/V.A.0.md`
**Date:** 2026-06-14
**Mode:** Read-only — no fix proposals, no code modifications.

---

## Files read

| Path | Lines | Notes |
|---|---|---|
| `docs/phases/phase-V.A.md` | 183 | full read |
| `crucible_core/src/crucible/ports/vault/vault_note_writer.py` | 75 | full read |
| `crucible_core/src/crucible/services/daily_chimera_service.py` | 349 | full read |
| `crucible_core/src/crucible/core/config.py` | 758 | full read |
| `crucible_core/src/crucible/core/schemas.py` | ~330 | re-export header only; Artifact/ToolOutput live in oligo |
| `crucible_core/src/oligo/core/schemas.py` | lines 132–183 | Artifact + ToolOutput class bodies |
| `astrocyte/src-tauri/src/lib.rs` | lines 1117–1188 | vault_contains_path + open_vault_note |
| `astrocyte/src-tauri/src/config.rs` | lines 30–40 | vault_root field |
| `vault://templates/Tpl_knowledge.md` | ~60 | full read |
| `vault://templates/Tpl_thought.md` | ~40 | full read |
| `vault://templates/Tpl_insight.md` | ~40 | full read |
| `vault://templates/Tpl_action.md` | ~40 | full read |
| `vault://Thoughts/Thought-memory bench-dataset source.md` | ~50 | frontmatter + body sample |
| `vault://Thoughts/Thought-memory bench-implementation pitfalls.md` | ~80 | frontmatter + body sample |
| `vault://Thoughts/Thought-memory bench-mix arch.md` | ~100 | frontmatter sample |
| `vault://Knowledge/2506.06326v1-MemoryOS_Deep_Read.md` | ~40 | frontmatter + body sample |
| `vault://01_Deep_Reads/2402.07630v3-GRetriever_Deep_Read.md` | ~40 | frontmatter + body sample |
| `vault://Insight/Dynamic empirical study on agent memory.md` | ~40 | frontmatter + body sample |
| `vault://Insight/Anti-patterns/The Graveyard of Ideas @1.md` | ~15 | frontmatter |
| `vault://Insight/Meta-theory/The Ontological Necessity of External Frameworks.md` | ~30 | frontmatter |
| `vault://dn+1.md` | ~60 | body sample (vault root level note) |
| `vault://dn+2.md` | ~30 | body sample (vault root level note) |

Directory listings (grep/ls, not full reads):
- `vault://` root, `vault://01_Deep_Reads/`, `vault://inbox/Must_Read/`, `vault://inbox/Skim/`
- `vault://Knowledge/`, `vault://Thoughts/`, `vault://Insight/`, `vault://Decision/`, `vault://templates/`
- `crucible_core/src/crucible/` full glob

---

## Findings

| Q# | Driving sprint | Question | Answer | Evidence | Risk |
|---|---|---|---|---|---|
| Q1 | V.A.0 | What is the actual vault_root path used at runtime? | Resolved from `~/.chimera/config.toml` via `ChimeraConfig.system.vault_root`; confirmed in use as `D:\MAS\project_chimera_vault` (sibling to repo). | `crucible_core/src/crucible/core/config.py:129` (`vault_root: Path | None`), `config.py:392` (_require_vault_root validator), `astrocyte/src-tauri/src/lib.rs:1158–1166` (runtime read) | Low |
| Q2 | V.A.0 | What frontmatter conventions are already in use across vault note types? | Three distinct conventions coexist: (1) PaperMiner-generated deep-read notes use `type: knowledge`, `chimera_status`, `arxiv_id`, `tags`, `graph_edges.used_by/related_work`; (2) Thought/Insight notes created via Tpl_ templates use `type`, `id`, `title`, `tags`, `chimera_status`, `graph_edges` with sub-fields; (3) older Insight/Meta-theory notes use only `created`, `tags`, `status`, `aliases` — no `type` field. | See frontmatter dumps below | Med |
| Q3 | V.A.0 | Do existing notes have typed edge fields (derives_from, supersedes, etc.)? | No V.A target edge names (`derives_from`, `supersedes`, `contradicts`, `dead_ends`) exist anywhere in the vault. Current Tpl_ templates use `derived_from` (not `derives_from`), `promoted_to_insight`, `drives_decision`, `supported_by`, `evidence_base`, `used_in`, `based_on`, `triggered_by`, `impacts`. Grep for target names returned zero results. | grep `derives_from\|supersedes\|contradicts\|dead_ends` across vault → 0 hits; `vault://templates/Tpl_thought.md:10–14` (actual field names) | High |
| Q4 | V.A.0 | How many notes exist and what is the breakdown by folder? | 400 total `.md` files. By folder: `01_Deep_Reads/` = 18, `inbox/Must_Read/` = 49, `inbox/Skim/` = 311, `Knowledge/` = 1, `Thoughts/` = 3, `Insight/` (incl. subdirs) = 4 (+ 2 READMEs), `Decision/` = 0, `templates/` = 4, vault root = ~5 loose notes + `images/`. | `find vault -name "*.md" | wc -l` = 400; per-folder `ls | wc -l` | Low |
| Q5 | V.A.0 | What does VaultNoteWriter.write_knowledge_node produce? | Renders `obsidian_tpl/knowledge_node.j2` Jinja template; writes output to `inbox_folder / verdict_value / {fancy_basename}.md`. No `obsidian_tpl/` Jinja templates exist in the crucible_core glob — they are not checked into the repo (likely in `~/.chimera/skills/` or the playground). | `vault_note_writer.py:31–45`; Jinja templates not found in `crucible_core/src/crucible/**/*.j2` glob (only oligo prompt templates found) | High |
| Q6 | V.A.0 | What does VaultNoteWriter.write_deep_read_node produce? | Renders `obsidian_tpl/deep_read_node.j2` or `deep_read_survey_node.j2`; writes to `vault_root / 01_Deep_Reads / {stem}_Deep_Read.md` (or `_Survey_Atlas.md`). The actual frontmatter is defined by the missing Jinja templates; sampled output notes (`01_Deep_Reads/`) show `type: knowledge`, `chimera_status: deep_read`, `arxiv_id`, `tags`, `processed`, `source_md`, `graph_edges.used_by/related_work`. | `vault_note_writer.py:47–74`; `vault://01_Deep_Reads/2402.07630v3-GRetriever_Deep_Read.md:1–8` | High |
| Q7 | V.A.0 | What frontmatter do actual deep-read output files set? | Fields confirmed in sampled files: `type: knowledge`, `chimera_status: deep_read`, `arxiv_id`, `title`, `tags` (including verdict tag), `processed` (date), `source_md` (absolute path to cleaned MD). No `status`, no `architecture_types`, no `short_moniker` in frontmatter — those appear only in body text. | `vault://Knowledge/2506.06326v1-MemoryOS_Deep_Read.md:1–8`; `vault://01_Deep_Reads/2402.07630v3-GRetriever_Deep_Read.md:1–8` | Med |

---

### Q2 frontmatter dumps

**Sample 1 — Thought note (template-created):**
```yaml
---
type: thought
id: mix_arch_investigation
title: Thought-20260408162840
tags:
  - thought
  - 💭/draft
chimera_status: raw
graph_edges:
  derived_from:
    - "[[{{REFERENCE_PAPER_OR_KNOWLEDGE}}]]"
  promoted_to_insight: []
  drives_decision: []
---
```
Source: `vault://Thoughts/Thought-memory bench-mix arch.md:1–14`

**Sample 2 — Insight note (template-created, stub):**
```yaml
---
type: insight
id: "202604142226"
title: "{{INSIGHT_STATEMENT_ONE_SENTENCE}}"
confidence: "{{CONFIDENCE: High/Med/Low}}"
domain: "{{DOMAIN}}"
tags: [insight, mental_model, axiom]
status: [Active, Challenged, Falsified]
graph_edges:
  supported_by: []
  evidence_base: []
  used_in: []
---
```
Source: `vault://Insight/Dynamic empirical study on agent memory.md:1–14`

**Sample 3 — Deep-read output note (PaperMiner-generated):**
```yaml
---
type: knowledge
chimera_status: deep_read
arxiv_id: "2402.07630v3-GRetriever"
title: "2402.07630v3-GRetriever"
tags: [deep_read, "📄/anatomist"]
processed: "2026-04-10"
source_md: "D:/MAS/crucible_core/playground/md_clean/2402.07630v3-GRetriever.md"
---
```
Source: `vault://01_Deep_Reads/2402.07630v3-GRetriever_Deep_Read.md:1–9`

| Q# | Driving sprint | Question | Answer | Evidence | Risk |
|---|---|---|---|---|---|
| Q8 | V.A.0 | What wikilink patterns exist in vault body text? | Two patterns: `[[NoteTitle]]` for untyped cross-references (e.g. `[[2603.16496v1-AdaMem_Deep_Read]]`) and `![[filename.png]]` for embedded images. No typed wikilinks (e.g. `[[note\|alias]]` used for edge semantics). All typed edge intent is expressed via frontmatter `graph_edges` sub-fields, consistent with V.A design decision. | `vault://Thoughts/Thought-memory bench-implementation pitfalls.md:32` (`[[2603.16496v1-AdaMem_Deep_Read]]`); `vault://Thoughts/Thought-memory bench-mix arch.md:35` (`![[Pasted image...]]`) | Low |
| Q9 | V.A.0 | Do any Knowledge Nodes (type=knowledge) exist with the full target ontology fields (type, status, arxiv_id, short_moniker, architecture_types, tags)? | No. Existing `type: knowledge` notes (deep-reads) have `chimera_status: deep_read` but lack `status`, `short_moniker`, and `architecture_types` frontmatter fields specified in V.A design decisions. The `Knowledge/` folder contains only 1 note; all 18 deep-reads are in `01_Deep_Reads/`. | `vault://Knowledge/2506.06326v1-MemoryOS_Deep_Read.md:1–8`; `vault://01_Deep_Reads/2402.07630v3-GRetriever_Deep_Read.md:1–8`; phase-V.A.md:86–91 (target schema) | Med |
| Q10 | V.A.0 | Are there any T/I/D nodes already in the vault? | T (Thought): 3 notes in `vault://Thoughts/`. I (Insight): 4 notes in `vault://Insight/` — 1 uses `type: insight` template frontmatter (stub, body empty), 3 older notes use legacy frontmatter (`created`, `status: Permanent`) with no `type` field. D (Decision): 0 notes — `vault://Decision/` folder exists but is empty. | `ls vault://Thoughts/` = 3; `ls vault://Insight/` + subdirs = 4 + 2 READMEs; `ls vault://Decision/` = 0 | Med |
| Q11 | V.A.0 | What does daily_paper_pipeline currently return, and what is the gap to the V.A.2 target? | Returns a plain `str` summary: `"Daily pipeline completed. new_pdfs={n} ingested={n} batch_total={n} must_read={n} skim={n} reject={n} errors={n} telegram=yes/no"`. No `ToolOutput`, no `Artifact` list, no per-paper structured data. The `_render_daily_report` builds a rich Telegram HTML message but this is side-channel (notifier only) — it never surfaces as a `ToolOutput`. | `daily_chimera_service.py:152–158` (return statement); `daily_chimera_service.py:258–348` (`_render_daily_report` — Telegram only) | High |
| Q12 | V.A.0 | Does the Artifact/ToolOutput infrastructure from Phase III.C exist and is it usable for V.A.2? | Yes. `Artifact(kind, path, metadata)` and `ToolOutput(text, artifacts)` are defined in `oligo/core/schemas.py` and re-exported from `crucible.core.schemas`. `ExecutedToolResult` also carries `artifacts: list[Artifact] | None`. Infrastructure is ready; `daily_paper_pipeline` just needs to wrap its return in `ToolOutput` with one `Artifact` per paper. | `crucible_core/src/oligo/core/schemas.py:132–163`; `crucible_core/src/crucible/core/schemas.py:7–18` (re-export block) | Low |
| Q13 | V.A.0 | What Astrocyte vault interaction surface exists, and what is missing for V.A.4? | `open_vault_note(path)` Tauri command exists: validates path is inside vault_root (traversal guard + canonicalize), then opens via `obsidian://open?path=...` URI. No Tauri commands for note creation, staging write, or promote/reject. The `vault_root` is read from `AstrocyteState` (loaded from `~/.chimera/config.toml`). | `astrocyte/src-tauri/src/lib.rs:1149–1188` (`open_vault_note`); `astrocyte/src-tauri/src/config.rs:30` (`vault_root` field); grep for `create_note\|write_note\|staging` → 0 hits | Med |
| Q14 | V.A.0 | Does a `docs/staging/` directory exist, and is any staging protocol already in place? | No. `docs/staging/` does not exist. No PENDING_REVIEW lifecycle, no promote/reject mechanism, no candidate node format. The staging protocol is fully greenfield for V.A.3. | `ls docs/staging` → `NO_STAGING_DIR`; grep for `PENDING_REVIEW\|staging` across repo → 0 hits | Low |

---

## Cross-references discovered

- **Artifact / ToolOutput (oligo.core.schemas)**: Defined at `crucible_core/src/oligo/core/schemas.py:132–163`; re-exported from `crucible_core/src/crucible/core/schemas.py:7–18`. Used by V.A.2 (structured pipeline output) and V.A.4 (node creation response). Already in production for Phase III.C tool results.
- **vault_root config key**: Declared as `SystemConfig.vault_root` at `crucible_core/src/crucible/core/config.py:129`; enforced non-None by validator at `config.py:392`; read by Astrocyte at `astrocyte/src-tauri/src/config.rs:30` and `lib.rs:1158–1166`. Single source of truth for all vault path resolution.
- **open_vault_note Tauri command**: Implemented at `astrocyte/src-tauri/src/lib.rs:1149–1188`. Path-traversal guard + canonicalize check. This is the only existing vault-facing Tauri command; V.A.4 must add `create_staging_node`, `promote_node`, `reject_node` alongside it.
- **inbox_folder vs vault_root split**: `VaultNoteWriter.write_knowledge_node` writes to `inbox_folder / verdict /` (`vault_note_writer.py:40`), while `write_deep_read_node` writes to `vault_root / 01_Deep_Reads /` (`vault_note_writer.py:69`). These are two separate config keys, both in `SystemConfig`. V.A.2 Artifact `vault_path` must resolve against the correct base depending on which writer produced the note.
- **Jinja obsidian_tpl templates**: Referenced by `VaultNoteWriter` at `vault_note_writer.py:34,59` but not found in the repo glob (`crucible_core/src/**/*.j2`). They are almost certainly in `~/.chimera/skills/` (the configured `skills_dir`). V.A.1 template authoring must account for this out-of-repo location.
- **Tpl_ Obsidian templates vs V.A target schema**: The vault's `templates/` folder contains `Tpl_knowledge.md`, `Tpl_thought.md`, `Tpl_insight.md`, `Tpl_action.md`. These are Obsidian Templater templates (for manual use). Field names diverge from V.A design spec in multiple places (see Notable Cross-Findings §1). V.A.1 must reconcile or supersede them.
- **Decision folder is empty**: `vault://Decision/` exists as a folder but contains 0 notes. D Nodes have never been created. No existing convention to anchor V.A.1 Decision template design.
- **Legacy Insight notes (no type field)**: Three Insight notes in `vault://Insight/Meta-theory/` and `vault://Insight/Anti-patterns/` predate the template system; they use `status: Permanent` and `created:` but no `type:` field. They will not be queryable by `vault_query type=insight` as designed.

---

## Notable cross-findings (no fix proposals — flagging for planning)

1. **Edge field name mismatch between Tpl_ templates and V.A design spec.** The phase-V.A.md design decisions (lines 86–99) specify edge names `derives_from`, `supersedes`, `contradicts`, `dead_ends` (for T/I/D nodes) and `used_by`, `related_work` (for K nodes). The actual Tpl_ templates in the vault use `derived_from`, `promoted_to_insight`, `drives_decision`, `supported_by`, `evidence_base`, `used_in`, `based_on`, `triggered_by`, `impacts`. These are entirely different field names. The 3 existing Thought notes and 1 existing Insight stub were created from the Tpl_ templates — they use the Tpl_ names. V.A.1 must decide authoritatively which names win, and V.A.5 `vault_query` must grep for whichever set is chosen. If V.A.1 adopts the spec names, the 4 existing typed nodes will be invisible to `vault_query` unless migrated.
   Evidence: `vault://templates/Tpl_thought.md:8–14`; `vault://templates/Tpl_insight.md:10–15`; `vault://templates/Tpl_action.md:9–13`; `phase-V.A.md:86–99`.

2. **Jinja obsidian_tpl templates are missing from the repo.** `VaultNoteWriter.write_knowledge_node` and `write_deep_read_node` both call `self.prompt_manager.render("obsidian_tpl/knowledge_node.j2", ...)` and sibling templates (`deep_read_node.j2`, `deep_read_survey_node.j2`). A glob of `crucible_core/src/**/*.j2` returns only four oligo prompt templates — none in `obsidian_tpl/`. These templates must live outside the repo (likely `~/.chimera/skills/`). V.A.1 will author new K/T/I/D templates; the planning batch must confirm where the Jinja templates are stored and whether `PromptManager` can resolve them from that location, before any V.A.2 work touches `VaultNoteWriter`.
   Evidence: `vault_note_writer.py:34,59`; glob `crucible_core/src/**/*.j2` → 4 hits, none in obsidian_tpl; `crucible_core/src/crucible/ports/prompts/jinja_prompt_manager.py` (exists but not read — template search path is the unknown).

3. **daily_paper_pipeline return value is a plain str with no per-paper data.** The pipeline collects per-paper data (score, arxiv_id, short_moniker, novelty, filename) in `_render_daily_report` at `daily_chimera_service.py:258–348`, but this data is consumed only for the Telegram HTML message and is not returned to the caller. The `run_daily_pipeline` return at line 152 is a single summary string with only aggregate counts. To implement V.A.2, the per-paper data must be bubbled up from `run_batch_filter`'s `BatchFilterStats.must_read_items` and wrapped in `ToolOutput(artifacts=[Artifact(...) per paper])`. The `BatchFilterStats` schema and what `must_read_items` actually contains must be verified before V.A.2 sprint planning.
   Evidence: `daily_chimera_service.py:152–158` (return); `daily_chimera_service.py:268–310` (per-paper data used only for Telegram); `oligo/core/schemas.py:132–163` (Artifact/ToolOutput ready).

4. **Vault note count distribution reveals inbox dominance.** Of 400 total notes, 360 (90%) are in `inbox/` (49 Must_Read + 311 Skim). Only 18 are in `01_Deep_Reads/`, 3 are Thought nodes, 1 is a Knowledge node (outside inbox), and 0 are Decision nodes. This confirms the vault is currently a paper triage dump, not yet a research production line. The K:T:I:D ratio target of 4:8:2:1 (Phase V.A sealing condition #5) is miles from current state (1:3:1:0). V.A.0 audit confirms this is expected — Phase V.A builds the ontology from scratch — but planning must account for the fact that 360 inbox notes are not typed and will never be queryable by `vault_query` without manual curation. This is a user workload implication, not a sprint blocker.
   Evidence: per-folder counts via `ls | wc -l`; `phase-V.A.md:79–81` (sealing condition #5).

---

## Audit complete

- 14 questions answered
- 38 file:line references
- 8 cross-references
- 4 notable cross-findings

**Suggested next:** `batch_planning` for Phase V.A.

---

*Generated by chimera-sprint-discipline phase_audit mode.*
