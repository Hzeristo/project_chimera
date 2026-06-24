# Project Chimera

A personal research operating system for a single user — a PhD student
working on Agent Memory systems. Not a framework, not a product. An
instrument, shaped to one researcher's cognitive workflow.

Chimera reduces the friction between reading papers, organizing knowledge,
and producing research. It does not try to do research autonomously. It
makes the human's research faster.

---

## Two Bodies

Chimera is two codebases under one roof:

- **Astrocyte** — the cognitive frontend. Rust + Svelte + Tauri desktop app.
  The surface where the user talks to the system, watches tasks run, and
  follows links back into their knowledge vault.

- **Crucible Core** — the engine room. Python. Contains:
  - **Oligo** — the agent: a ReAct loop that routes intent, calls tools,
    distills results, and answers in a chosen persona.
  - **PaperMiner** — the ingestion pipeline: arXiv → MinerU → Markdown →
    LLM filtering → Obsidian vault.

The two communicate over a stable SSE contract. Either can be replaced
without rewriting the other.

---

## The Vault Ontology

Knowledge lives in an Obsidian vault, structured as four node types — a
hierarchy that mirrors how research knowledge actually accumulates:

- **Knowledge Nodes** — immutable deep-read extractions from papers.
- **Thought Nodes** — mutable, append-only research observations and sparks.
- **Insight Nodes** — cross-verified conclusions, confirmed through discussion.
- **Decision Nodes** — the spine: directional research choices, time-ordered.

K, T, and I form a pyramid. D runs along it as a temporal axis. This
ontology is structurally aligned with the ARA (Agentic Research Artifact)
four-layer model — Chimera consumes that format without implementing its
platform.

---

## The Oligo Agent

A single user turn flows through four stages:

```
Router   →   Tool Execution   →   Wash   →   Final
(intent)     (parallel calls)     (distill)   (persona)
```

- **Router** is the cognitive subject. It classifies intent, decides
  whether the vault might hold the answer, formulates a retrieval strategy,
  and dispatches tools. It reasons in `<thinking>` blocks.
- **Tool Execution** runs registered tools, with concurrency partitioning
  by safety and per-tool timeouts.
- **Wash** is an intent-driven compressor — it distills tool output down to
  what the router actually needed, using a cheap model.
- **Final** is a thin persona layer. It rewrites the synthesis in voice.
  It does no fresh cognition.

Tools are explicit and bounded. The agent can do exactly what the tool
registry permits — no arbitrary command execution, all file operations
inside the vault boundary.

---

## Design Principles

**Instrument, not framework.** Frameworks optimize for generality across
unknown users. Chimera optimizes for fit to one. It can be ten times simpler
than a general agent framework while being ten times more useful to its user.

**Provider-agnostic.** Built on the OpenAI-compatible API surface. The
working model is a configuration choice — Claude, DeepSeek, or anything that
speaks the protocol. No SDK lock-in.

**Friction-driven.** Priorities come from logged friction during real use,
not from a roadmap. Features without a corresponding friction entry are
deferred. After each phase, the system is used before more is built.

**Layered harness.** The agent's prompt scaffolding is split into two kinds:
scaffolding that compensates for weak models (which thins as base models
improve) and boundaries that constrain a capable agent (which never go away).
The architecture lets the first kind be deleted without touching the second.

**Knowledge over narrative.** What matters is the structured knowledge a
research process produces, not the story of how it was produced.

---

## Architecture

Crucible Core follows strict domain-driven layering:

```
core/      schemas, config, platform — depends on nothing internal
ports/     external adapters (LLM, vault, arxiv, ingest, notify)
services/  business workflows
```

Dependencies flow one direction only: `core → ports → services`.

Prompts live as external templates (`.md.j2` for static, `.md` for
runtime-variable), composed by a `PromptComposer` with cache-aware
static/dynamic separation. No prompt text is hardcoded in source.

State and conventions are documented under `docs/` — a project memory layer
(`ROADMAP`, `FRICTION_LOG`, `ACCEPTED_PARTIALS`, `TECHNICAL_DEBT`,
`ARCHITECTURE/`) consumed by the development skills that maintain this repo.

---

## Status

Sealed through Phase V.A: the agent core is async (long-running tasks
suspend and resume on real completion instead of fabricating an answer),
and the Exocortex node ontology, staging protocol, and vault query are in
place. Next: an end-to-end smoke pass and a Use Week, then the next Phase V
sprint.

For the authoritative per-phase breakdown — sealed, planned, or deferred —
see [`docs/ROADMAP.md`](docs/ROADMAP.md).

This is a system under active, single-developer construction. It is built
to be used for four years, not shipped.

---

*Chimera is named after the synthesis it performs: stitching together
disparate models, tools, and knowledge into one coherent research organism.*