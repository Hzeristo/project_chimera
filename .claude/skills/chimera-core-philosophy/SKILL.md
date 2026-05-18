---
name: chimera-core-philosophy
description: Core design philosophy for Project Chimera, a personal research OS. Activate when working on Chimera codebase, discussing architecture decisions, or when scope decisions are being made. Reinforces single-user focus, friction-driven development, and anti-framework stance.
---

# Chimera Core Philosophy

## What Chimera Is

Chimera is a **personal research OS** for one user. Not a SaaS, not a framework, not a product for others. The user is a PhD student. The goal is cognitive leverage during research, not building something to distribute.

## What Chimera Is Not

- **Not a framework.** Frameworks optimize for generality across unknown users. Chimera optimizes for fit to one user.
- **Not a Claude Code clone.** Claude Code serves developers-at-large. Chimera serves academic research.
- **Not an OpenClaw clone.** Opneclaw act as general purpose OS agent. Chimera is a dedicated research OS.
- **Not SOTA chasing.** Chimera is "2024.5 architecture" — mature tech combined with novel integration, not bleeding-edge "2026.5" papers.

## Core Principles

### Friction-Driven Development
- Priorities come from **friction logs** (real usage pain), not benchmarks, not roadmaps.
- If a proposed feature doesn't address a logged friction, question why it's being built.
- After each phase, propose and run a **Use Week**: stop coding, use the system, log friction.

### Agent, Not Orchestrator
- Orchestrator: user writes the workflow, system executes.
- Agent: user states goal, system decomposes.
- **Chimera is an agent.** Never drift toward YAML-defined workflows.

### SOTA Plunder, Not SOTA Copy
- Read SOTA papers for **architectural insights**, not implementation blueprints.
- Copy the idea (e.g., Context-Folding's branch/return), discard the prompt templates and benchmark-specific details.
- Every SOTA borrow should ask: "What vulnerability does this paper not solve, and do we care about that vulnerability?"

### Lightweight Over Comprehensive
- Single-user means we can skip:
  - Multi-provider abstractions (OpenAI-compatible only)
  - Sandboxing / SSRF protection (local, no untrusted input)
  - Plugin SDKs / extension systems
  - Distributed tracing (structured logs suffice)
- Adding these "for safety" is SaaS thinking contamination.

## The Four-Layer Capability Model

Chimera distinguishes these four explicitly. They are not interchangeable:

| Layer | Meaning | Example |
|---|---|---|
| **Persona** | Conversational posture, voice, personality | "Reviewer Zero", "BB" |
| **Skill** | Task path template | "Deep read a paper with forensic angle" |
| **Tool** | Atomic capability | `search_vault`, `arxiv_miner` |
| **Lens** | Structured output schema for Optics pipeline | `ForensicLens`, `DecayLens` |

Never conflate these layers. If a "Skill" is actually defining output structure, it's a Lens. If a "Persona" is specifying tool chains, it's a Skill.

## When in Doubt, Reduce

If adding a capability feels like it's moving Chimera toward a framework shape, the default answer is no. The question to ask:

> "Does this solve a logged friction, or is it anticipating a problem the user doesn't have yet?"

Anticipated problems belong in `Phase IV` placeholder docs, not in the current sprint.
