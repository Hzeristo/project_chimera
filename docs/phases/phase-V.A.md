# Phase V.A — Exocortex Node Ontology & Research Production Line

**Status:** Planned
**Sealed predecessor:** IV.A (async agent core)
**Driving frictions:**
- friction-260611/260613 E1 residual: daily_paper_pipeline returns terse
  summary string, not structured ToolOutput with per-paper artifacts.Agent generates report from keywords, not from real data.
- friction-260611 E3 residual: tool results lack file paths; frontend
  can't link to vault notes from BB replies.
- No friction yet on K/T/I/D creation workflow — but Phase IV.A's
  async core makes it possible. This phase builds the consumption layer.

## Mission

Turn the Obsidian vault from passive storage into an active research
production line. Define the K/T/I/D node ontology as typed Markdown
templates with frontmatter schema. Make daily_paper_pipeline return
structured per-paper artifacts. Enable one-click node creation from
BB conversations. Build lightweight frontmatter-indexed vault query.

This is the phase where Chimera becomes usable for daily research,
not just daily debugging.

## Sprint Sequence

| Sprint | One-line goal | Status |
|---|---|---|
| V.A.0 | Audit: explore actual vault structure (project_chimera_vault/), existing node formats, frontmatter conventions, wikilink patterns, PaperMiner output templates, and gap between current vault state and target K/T/I/D ontology | Pending |
| V.A.1 | K/T/I/D node templates: Markdown + frontmatter schema for each type, aligned with ARA four-layer semantics; typed edge conventions (derives_from, supersedes, contradicts, dead_ends) | Pending |
| V.A.2 | daily_paper_pipeline structured output: returnToolOutput with per-paper Artifact (title, arxiv_id, verdict, vault_path); replace terse summary string | Pending |
| V.A.3 | Staging area: docs/staging/ protocol for BB-conversation-extracted candidate nodes; PENDING_REVIEW → PROMOTED / REJECTED lifecycle | Pending |
| V.A.4 | Astrocyte one-click node creation: hover BB message → create candidate Thought Node in staging; promote/reject from staging panel | Pending |
| V.A.5 | vault_query tool: frontmatter-indexed query by type/status/linked_to; ripgrep + frontmatter parse, no DB, no embedding | Pending |
| V.A.6 | Seal sprint: FINAL_CONTRACT V.A doc + E2E smoke + phase review | Pending |

Dependencies: V.A.0 precedes all (must understand actual vault before
designing templates). V.A.1 precedes V.A.2-V.A.5(templates are the
substrate). V.A.2 independent of V.A.3-V.A.5. V.A.4 depends on V.A.3
(staging protocol). V.A.5 depends on V.A.1 (frontmatter schema).
V.A.6 after all.

## Cross-Sprint Red Lines

-❌ NO embedding / vector DB / semantic search (Phase VI)
- ❌ NO PPR / graph random walk (Phase VI)
- ❌ NO automatic node creation without human review (staging +PENDING_REVIEW is the mechanism; human promotes)
- ❌ NO modification of existing vault notes without explicit user
  consent (new nodes only; existing notes are immutable unless user
  edits manually)
- ❌ vault_query uses ripgrep + frontmatter parse ONLY — no SQLite,
  no index file, no daemon. Query latency target< 2s on 1000 notes.
- ❌ K Nodes are IMMUTABLE after creation (append-only errata via
  linked T Nodes, never in-place edit)
- ❌ T Nodes are APPEND-ONLY (new observations appended, old content
  never modified)
- ❌ Do NOT introduce Obsidian plugin dependencies — all vault
  operations are file-level (read/write .md via Python)
- ❌ ARA alignment is STRUCTURAL (template fields map to ARA layers),
  NOT platform-level (no ARA Commons, no Seal Certificate)

## Hard Sealing Conditions

1. (Templates) K/T/I/D templates exist as .md files with typed
   frontmatter; Templater-compatible for manual Obsidian use.
   Each template has: type, status, created_at, linked nodes
   (typed edges via frontmatter list fields).
2. (Structured output) daily_paper_pipeline returns ToolOutput with
   ≥1Artifact per processed paper; each Artifact has title, arxiv_id,
   verdict, vault_path pointing to the actual filtered MD file.
   Verified: BB reply after pipeline lists real paper titles (not
   keywords from terse summary).
3. (Staging) A candidate T Node created from BB conversation lands
   in docs/staging/ with Status: PENDING_REVIEW. Promote moves it
   to the vault proper. Reject deletes it. Verified by manual flow.
4. (vault_query) Query "all K Nodes about memory" returns results
   in< 2s on current vault size. Results include title + path +
   frontmatter excerpt. Verified by live query.
5. (Node ratio) After one week of use, vault contains nodes in
   roughly K:T:I:D ≈ 4:8:2:1 ratio — confirms ontology is being
   used, not just defined. (Verified at Phase V.A review, not at
   sprint level.)

## Design Decisions

- **K/T/I/D ontology (from ST discussions, not re-derivable)**:
  - Knowledge Node: immutable deep-read extraction from papers.Frontmatter: type=knowledge, status=deep_read, arxiv_id, short_moniker,
    architecture_types, tags. Body: structured analysis per Optics lens.
  - Thought Node: mutable (append-only) research observation.
    Frontmatter: type=thought, status=active|dead_end|pivot,
    derives_from (typed edge to K/T nodes), lesson (if dead_end).Body: timestamped observations, never edited in-place.
  - Insight Node: cross-verified conclusion (confirmed with advisor).
    Frontmatter: type=insight, status=cross_verified, synthesizes
    (typed edge to T nodes), verified_with, actionable.
    Body: the insight + its implications + actionable next step.
  - Decision Node: directional research choice.
    Frontmatter: type=decision, status=active|superseded,
    depends_on (typed edge to I nodes), dead_ends (typed edge to
    abandoned T nodes), rationale.
    Body: the decision + why + what was abandoned.

- **Typed edges via frontmatter, not wikilink syntax (ST2026-05-29)**:
  Wikilinks [[Target]] are untyped. Typed edges are frontmatter list
  fields: derives_from: ["[[K-MemGPT]]"], supersedes: ["[[T-old-idea]]"].
  This enables vault_query to filter by edge type without parsing
  wikilink context. Wikilinks in body text remain untyped (free prose).

- **ARA alignment is structural (ST 2026-05-27)**:
  K Node≈ ARA /logic/claims (immutable extracted knowledge).
  T Node ≈ ARA /trace (observations, including dead_ends).
  I Node ≈ ARA /logic/solution (validated conclusions).
  D Node ≈ ARA /trace decision+pivot nodes (directional choices).
  No ARA Compiler integration in V.A (manual alignment via templates).

- **daily_paper_pipeline structured output (friction-driven)**:
  Current: returns "new_pdfs=0ingested=3 must_read=0 skim=1reject=2".
  Target: returns ToolOutput(text=summary, artifacts=[Artifact per paper]).
  Each Artifact: kind="vault_note", path=filtered_md_path, title=paper_title,
  metadata={arxiv_id, verdict, score}.
  This reuses Phase III.C's FC.1ToolOutput/Artifact infrastructure.

- **Staging is human-gated (L2 philosophy)**:
  BB conversations may surface candidate T/I/D nodes. These go to
  docs/staging/ as PENDING_REVIEW, NOT directly to the vault.
  User reviews in Obsidian (or Astrocyte staging panel) and promotes
  or rejects. No automatic promotion. Human review is the feature.

- **vault_query is dumb-but-fast (no intelligence)**:
  ripgrep over frontmatter YAML fields. No ranking, no relevance
  scoring, no embedding similarity. Just: "give me all notes where
  type=thought AND status=dead_end AND derives_from containsMemGPT".
  This is grep, not search. Intelligence comes from the user's query
  formulation + Router's intent recognition (already in IV.A).

## V.A.0 Audit — Special Instructions

CRITICAL: Claude Code has NEVER seen the actual Obsidian vault.
The vault is at a SIBLING directory to the repo, not inside it.

The audit MUST:

1. Explore the vault directory structure.The vault root is declared in config.toml under [system] vault_root.
   Read config.toml to find the path, then explore:
   - Top-level directory listing (folder names, counts)
   - Sample3-5 existing Knowledge Nodes (deep read results)
   - Sample any existing Thought/Insight/Decision nodes if they exist
   - The frontmatter conventions already in use (what fields? what values?)

2. Explore PaperMiner output that feeds the vault.
   - What does VaultNoteWriter.write_knowledge_node produce?
     (file: crucible_core/src/crucible/ports/vault/vault_note_writer.py)
   - What does VaultNoteWriter.write_deep_read_node produce?
   - What frontmatter does it set? What body structure?
   - Sample an actual output file in the vault.

3. Identify the gap between current vault state and target ontology.
   - Do existing notes already have type/status frontmatter?
   - Are there any typed edges (derives_from etc.) in existing notes?
   - What wikilink patterns exist in the body text?
   - How many notes exist? What's the breakdown by folder/type?

4. Explore Astrocyte's vault interaction surface.
   - How does open_vault_note work? (already implemented in III.C)
   - Are there any existing Tauri commands for vault note creation?
   - What would a "create candidate node" Tauri command need?

5. Report: docs/audits/V.A.0.md
   Structure: per-question tables with file:line (for code) or
   file:path (for vault notes). Include2-3 sample frontmatter
   dumps from existing vault notes.

   Do NOT propose templates — that's V.A.1 scope, user-defined.

## Out of Scope (→ Phase VI+)

- Embedding-based semantic search / vector DB
- PPR / graph random walk over vault
- Heterogeneous layered KG-RAG
- ARA Compiler integration (manual template alignment only)
- Insight → Claude Code bridge (Phase V.E)
- Gravedigger / OpenReview miner
- Automatic node creation without human review
- Obsidian plugin development
