# Node Ontology — K/T/I/D Schema

Authoritative reference for the four vault node types. Source of truth for frontmatter field names, edge names, and lifecycle rules. Generated at Phase V.A.1.

---

## Node Types

### K — Knowledge Node
Immutable record of a processed paper. Written by `VaultNoteWriter.write_knowledge_node`.

```yaml
type: knowledge
status: unverified          # → verified after manual review
arxiv_id: "2501.12345"
title: "..."
authors: "..."
year: "2025"
score: 8
verdict: "must_read"
short_moniker: "FooBarNet"
source_md: "papers/..."     # repo-relative path to the filtered MD
processed: "2026-01-15"
tags: [knowledge]
graph_edges:
  derives_from: []
  supersedes: []
  contradicts: []
```

### T — Thought Node
Append-only working hypothesis or observation. Created via staging protocol (V.A.3).

```yaml
type: thought
status: active              # active | dead_end
title: "..."
created: "2026-01-15"
tags: [thought]
graph_edges:
  derives_from: []
  supersedes: []
  contradicts: []
  dead_ends: []
lesson: ""                  # filled when status → dead_end
```

### I — Insight Node
Synthesized claim, cross-verified across ≥2 sources.

```yaml
type: insight
status: cross_verified      # cross_verified | retracted
title: "..."
created: "2026-01-15"
tags: [insight]
graph_edges:
  synthesizes: []
  verified_with: []
  derives_from: []
  supersedes: []
  contradicts: []
actionable: ""
```

### D — Decision Node
Greenfield. Captures a research or design decision with rationale.

```yaml
type: decision
status: active              # active | superseded
title: "..."
created: "2026-01-15"
tags: [decision]
graph_edges:
  depends_on: []
  dead_ends: []
  supersedes: []
  contradicts: []
rationale: ""
```

---

## Authoritative Edge Names

These names are canonical. The 2026-06-14 migration eliminated all legacy variants.

| Edge          | Meaning                                  | Used by     |
|---------------|------------------------------------------|-------------|
| `derives_from`| This node follows from another           | K, T, I     |
| `supersedes`  | This node replaces another               | K, T, I, D  |
| `contradicts` | This node conflicts with another         | K, T, I, D  |
| `dead_ends`   | Paths this node terminates               | T, D        |
| `synthesizes` | Source nodes this insight integrates     | I           |
| `verified_with` | Nodes that cross-verify this insight   | I           |
| `depends_on`  | Decisions this decision requires         | D           |

Legacy names (`derived_from`, `promoted_to_insight`, `promotes_to`, `linked_to_paper`) are invalid. Do not use them.

---

## Lifecycle Contracts

**K nodes — IMMUTABLE.** After `write_knowledge_node` writes the file, the frontmatter and body are never modified by any automated process. Status transitions (`unverified` → `verified`) are manual.

**T nodes — APPEND-ONLY.** The `## Log` section accumulates dated entries; prior entries are never edited. `status: dead_end` + `lesson:` may be added at the bottom, never retroactively.

**I and D nodes** — editable by manual authorship only. No automated process writes to existing I/D files.

---

## Vault Templates Deployment (Manual)

Templater-compatible `.md` templates live in the repo at:

```
crucible_core/prompts/obsidian_tpl/Tpl_{knowledge,thought,insight,decision}.md
```

These must be **manually copied** into the vault `templates/` directory by the user before the Phase V.A E2E smoke. The agent does not write to `vault://templates/`.

Jinja2 render templates (used by `VaultNoteWriter` and `PromptManager`):

```
crucible_core/prompts/obsidian_tpl/{knowledge,deep_read,thought,insight,decision}_node.j2
```
