# Prompt Middleware (`PromptComposer`)

**Phase:** III.B.1 (MW.0–4) · III.B.3 IR.4 (`xml_structured`) | **Status:** Active  
**Updated:** 2026-05-17 (against commit `48b2b2a`)

## Purpose

`PromptComposer` merges router- and final-stage system prompts from named `PromptComponent` rows: fixed ordering by `priority`, optional per-agent `active_ids`, and split output into `(stable_section, dynamic_section)` so mostly-static text can ride ahead of per-request deltas. Related history hygiene and stripping live in `TextSanitizer`; wash system prompts intentionally stay outside registration (accepted partial MW.1.1).

## Architecture

```text
schemas.PromptStage + PromptComponent
        │
get_prompt_composer() → PromptComposer.register / compose
        │
ChimeraAgent._prompt_context() supplies format keys ({tool_list}, {timestamp}, …)
        │
├─► _compute_active_router_components() → compose(ROUTER) → router system message
└─► _compute_active_final_components()  → compose(FINAL) → final stream system
```

Stable vs dynamic join at call sites (`stable` then `dynamic`, trimmed): router `480:511:project_chimera/crucible_core/src/oligo/core/agent.py`, final `517:535:project_chimera/crucible_core/src/oligo/core/agent.py`

## API / Schema

| Symbol | Signature / role | Location |
|--------|------------------|-----------|
| `PromptStage` | `ROUTER`, `FINAL`, `BOTH`, `MESSAGE_INJECTION` | `529:535:project_chimera/crucible_core/src/crucible/core/schemas.py` |
| `PromptComponent` | `id`, `stage`, `priority`, `cacheable`, `renderer` (`text`|`xml_structured`), `template` | `541:572:project_chimera/crucible_core/src/crucible/core/schemas.py` |
| `PromptComposer.register` | `component: PromptComponent` → raises on duplicate id | `245:249:project_chimera/crucible_core/src/oligo/core/prompt_composer.py` |
| `PromptComposer.compose` | `(stage, context, active_ids)` → `(stable_section, dynamic_section)` | `251:307:project_chimera/crucible_core/src/oligo/core/prompt_composer.py` |
| `get_prompt_composer` | lazy singleton + `_register_default_components` | `322:326:project_chimera/crucible_core/src/oligo/core/prompt_composer.py` |

### Registered default components (composer global registry)

Rendered order is **`priority` descending** (`284:284:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`). **`cacheable=True`** entries append to **`stable_parts`**; **`False`** append to **`dynamic_parts`** (`288:304:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`).

| `id` | `stage` | `priority` | `cacheable` | Template summary |
|------|---------|------------|-------------|-------------------|
| `router_core` | ROUTER | 100 | Yes | Chimera router role; XML `<tool_call>` + legacy `<CMD:…>` specs; naming / multi-call policy (`31:61:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`) |
| `router_tool_registry` | ROUTER | 90 | Yes | `Available tools:\n{tool_list}\n\n` + footer rules/meta (`ROUTER_POST_TOOLS`) |
| `router_skill_directive` | ROUTER | 80 | Yes | Routed skill override block (`"[USER SKILL DIRECTIVE…]:\n{skill_override}"`) |
| `retrieval_context_demo` | ROUTER | 15 | Yes | **`renderer="xml_structured"`** fixed demo dict (example retrieval branch) (`364:380:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`) |
| `final_system_core` | FINAL | 100 | Yes | `{system_core}` base persona text |
| `final_skill_directive` | FINAL | 80 | Yes | `[SKILL DIRECTIVE]` + `{skill_override}` |
| `final_persona_override` | FINAL | 60 | Yes | `[PERSONA OVERRIDE]` + `{persona}` |
| `final_authors_note` | FINAL | 40 | Yes | `[AUTHOR'S NOTE]` + `{authors_note}` |
| `final_guardrail` | FINAL | 10 | Yes | `[EXECUTION CONTEXT]` forbid pseudo tool syntax after tools (`18:26:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`) |
| `dynamic_timestamp` | BOTH | 5 | **No** | ISO timestamp via `{timestamp}` (`428:435:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`) |

Router `tool_list` text is delegated to `_render_tool_list(...)` (`144:181:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`) whenever `{tool_list}` is formatted through `router_tool_registry`.

> **Activation note.** `retrieval_context_demo` is registered but **not** included in `_compute_active_router_components` (`433:446:project_chimera/crucible_core/src/oligo/core/agent.py`); it only appears when callers pass matching `active_ids` (`145:157:project_chimera/crucible_core/tests/oligo/test_prompt_composer.py`). Treat it as a demo hook—not live retrieval content until Phase IV ingestion exists.

### `stable_section` vs `dynamic_section`

- **`stable_section`**: join of **`cacheable=True`** component bodies (`306:307:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`). Field doc: mark content that **should behave as reusable prefix**, vs per-request/session noise (`551:557:project_chimera/crucible_core/src/crucible/core/schemas.py`).
- **`dynamic_section`**: join of **`cacheable=False`** parts (today: `dynamic_timestamp`).
- **Rationale.** Splitting matches the schema intent that `cacheable` marks **prompt-cache-friendly** chunks vs per-turn noise (`551:557:project_chimera/crucible_core/src/crucible/core/schemas.py`). `ChimeraAgent` concatenates `stable` then `dynamic` (`488:491:project_chimera/crucible_core/src/oligo/core/agent.py`, `525:528:project_chimera/crucible_core/src/oligo/core/agent.py`). Whether the remote LLM client exploits that split for prefix KV caching is implementation-dependent.

### `xml_structured` renderer (IR.4) · Phase IV reservation

When `renderer == "xml_structured"`, `template` **must be a `dict`**; render path builds `<structured>…</structured>` XML via ElementTree (`230:236:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`), `(289:294:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`). **Inbound LLM replies are never parsed here** (`555:557:project_chimera/crucible_core/src/crucible/core/schemas.py`).

Production-scale **PPR / graph retrieval payloads** feeding router prompts remain **out of Phase III.C scope** (`20:20:project_chimera/docs/phases/Phase-III.C.md`); this demo validates the **`xml_structured` plumbing only** (`364:380:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`).

### Three-layer `TextSanitizer`

| Layer | Method | Responsibility | Code |
|-------|--------|----------------|------|
| 1 · reasoning tags | `strip_reasoning_tags` | Removes `<thinking>…`, `<think>…`, orphan opens | `143:182:project_chimera/crucible_core/src/oligo/core/text_sanitizer.py` |
| 2 · routing DSL outside code | `strip_tool_syntax_in_visible` | Strips `<CMD>`, `<PASS>`, `<tool_call>` **outside fenced/inline spans** (`_rebuild_stripping_outside_code`) | `192:196:project_chimera/crucible_core/src/oligo/core/text_sanitizer.py` |
| 3 · outbound history | `sanitize_messages_history` | Drops orphan `role="tool"` rows, runs `_content_sanitize_str` (`strip_tool_syntax`, then reasoning for allowed roles), caps assistant chars | `215:250:project_chimera/crucible_core/src/oligo/core/text_sanitizer.py` |

`_apply_history_sanitizer_to_messages` marks **layer‑3 insertion point before LLM** (`513:515:project_chimera/crucible_core/src/oligo/core/agent.py`).

## Decision Points

- **`PromptStage.BOTH`** includes a component whenever the **target stage is ROUTER or FINAL** (`315:316:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`). Use for identical cross-stage tails (currently `dynamic_timestamp`).
- **`PromptStage.FINAL` / ROUTER`** match exactly; **asking `compose(BOTH)`** matches only components whose **`stage == BOTH`** (`317:318:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`)—not used by `ChimeraAgent`.
- Router **drops any template mentioning `{persona}`** (`271:279:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`) even if mis-registered—a guard against persona bleed in probe phase.
- **Wash stays out of composer** intentionally (`31:37:project_chimera/docs/ACCEPTED_PARTIALS.md`): `_wash_tool_result` builds washer system text inline (`936:961:project_chimera/crucible_core/src/oligo/core/agent.py`).
- **Tool registry caching caveat:** `router_tool_registry` stays `cacheable=True` though `{tool_list}` bytes change whenever tool metadata changes (**MW.2.1**) (`35:37:project_chimera/docs/ACCEPTED_PARTIALS.md`).

## Checklist: Adding to This Subsystem

1. Add a `PromptComponent(...)` row inside `_register_default_components` (`330:436:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`) with **`id` uniqueness** enforced by `register` (`246:249:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`).
2. Choose **`stage`** (`ROUTER`, `FINAL`, or `BOTH`) using `_component_matches_stage` semantics (`314:319:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`); router-only persona strings still must **avoid `{persona}`** placeholders or keep them FINAL-only (`271:279:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`).
3. Set **`priority`** relative to neighboring fragments (tie-break: higher renders earlier) (`548:549:project_chimera/crucible_core/src/crucible/core/schemas.py`).
4. Set **`cacheable`**: timestamps / volatile tokens ⇒ `False` (`551:557:project_chimera/crucible_core/src/crucible/core/schemas.py`).
5. If structured injection: `renderer="xml_structured"` + dict `template` validated by pydantic (`555:569:project_chimera/crucible_core/src/crucible/core/schemas.py`) and runtime (`289:293:project_chimera/crucible_core/src/oligo/core/prompt_composer.py`).
6. Wire **`active_ids`** in `_compute_active_router_components` / `_compute_active_final_components` (`433:461:project_chimera/crucible_core/src/oligo/core/agent.py`) whenever the fragment must ship to `ChimeraAgent`; omit if test-only/offline.

## Known Issues

- **MW.1.1:** Wash prompts built outside composer (`31:37:project_chimera/docs/ACCEPTED_PARTIALS.md`) — cite `936:961:project_chimera/crucible_core/src/oligo/core/agent.py`.
- **MW.2.1:** `tool_list`/registry drift breaks prefix identity despite `cacheable=True` (`35:37:project_chimera/docs/ACCEPTED_PARTIALS.md`).
- **DEBT‑001 / MW.4.2:** six async tests missing asyncio markers skipped silently (`39:41:project_chimera/docs/ACCEPTED_PARTIALS.md`) — tracked **`DEBT‑001`** (`13:14:project_chimera/docs/TECHNICAL_DEBT.md`).
- **DEBT‑005:** Router tooling tier occasionally falls compact without extreme tool counts; observe after Phase III.C (`17:17:project_chimera/docs/TECHNICAL_DEBT.md`). *(Threshold tuning—not a Composer defect but affects `{tool_list}`.)*

## Cross-references

- [`TOOL_PROTOCOL.md`](./TOOL_PROTOCOL.md) — XML / `<CMD:…>` surface area referenced by `router_core`; argument repair consumes parsed calls after stripping (`542:549:project_chimera/crucible_core/src/oligo/core/agent.py`).
- [`INTENT_AND_DEGRADATION.md`](./INTENT_AND_DEGRADATION.md) — Tiered `_render_tool_list` budgets interplay with `_ROUTER_SYSTEM_PROMPT_MAX_CHARS` (`72:72:project_chimera/crucible_core/src/oligo/core/agent.py`, `483:511:project_chimera/crucible_core/src/oligo/core/agent.py`).
- [`CONFIG_SCHEMA.md`](./CONFIG_SCHEMA.md) — where LLM personas / timeouts feeding `{system_core}` originate.
