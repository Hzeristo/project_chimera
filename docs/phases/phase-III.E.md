# Phase III.E — Oligo Orchestration Primitives

**Status:** Sealed 2026-06-11
**Sealed predecessor:** EXT
**Driving frictions:**
- friction-260526 (deep read 污染主 context — tool calling 增多后的连带问题)
- ST 2026-05-28 (反复 rebuttal 中,失效的 proposal 污染后续推理)
- Phase IV 前置依赖 (deep_research / Claude Code 桥 需要 subagent + babysitting)

## Mission

为 Oligo 建立三个上下文编排原语,使 Oligo 从单线程剧场升级为多上下文
编排器。这是 Phase IV 的前置基础设施,不是 Phase IV 本身。

## Sprint Sequence

| Sprint | One-line goal | Status |
|---|---|---|
| III.E.0 | Audit: 当前 theater loop 的 context 累积点 + ChimeraAgent 实例化链路 | Done |
| III.E.A | fork_subagent + run_isolated: context 隔离子任务,只回 summary | Done |
| III.E.C | context archival: 语义/意图驱动的 segment tombstone,原文存 audit log | Done |
| III.E.B | Claude Code babysitting: subprocess + stdout 流式转 ActiveTaskPanel | Done |

Dependencies: III.E.0 precedes all. A and C are context-management family
(do consecutively). B is subprocess-based (independent, can be last).

## Cross-Sprint Red Lines

- ❌ NO arbitrary command execution tool (no run_shell, no run_pwsh)
- ❌ NO subagent nesting (max depth 1 — subagent cannot fork another subagent)
- ❌ NO parallel subagent (sequential fork-merge only)
- ❌ NO interleaved-CoT via provider API (prompt-level <thinking> is sufficient)
- ❌ Subagent shares NO mutable state with parent — returns summary only
- ❌ Archived segments are tombstoned, NOT deleted — original preserved in audit log
- ❌ Reuse existing TaskService event bus for babysitting — do NOT build new infra
- ❌ All file operations stay within vault_root / project boundaries

## Hard Sealing Conditions

1. Subagent isolation: fork_subagent given a 50K-token prompt returns a
   summary ≤ 4096 chars, and the parent agent's message list does NOT
   contain the 50K content (verified by unit test — original "50K paper
   via real router path" condition downgraded 2026-06-11: no router
   trigger path + no real long-result source available at phase time)
2. Context archival: an archived segment is replaced by a tombstone in
   live context AND recoverable from audit log (verified by un-archive)
   — manually verified 2026-06-11
3. Babysitting: invoking a Claude Code subprocess shows live progress in
   ActiveTaskPanel and produces a DEAD_END candidate node on failure
   (verified by live test with a deliberately failing task)
   — manually verified 2026-06-11

## Design Decisions (from ST discussion, not re-derivable)

- **Three primitives, one shift (ST 2026-05-28)**: subagent (context
  isolation), archival (context purification), babysitting (subprocess
  monitoring). All three manage "what is in the main context" or "what
  the agent is waiting on." Together they turn the single-thread theater
  into a multi-context orchestrator.

- **Subagent vs TaskService distinction (ST 2026-05-28)**: subagent is
  synchronous + context-isolated + returns summary (seconds-scale, e.g.
  deep read). TaskService is asynchronous + separate process + returns
  task_id (minutes-scale, e.g. daily_pipeline). They are NOT the same
  mechanism. fork_subagent is a new ChimeraAgent instance reusing the
  same llm/wash/vault clients with its own messages list.

- **Archival is semantic/intent-driven, not token-driven (ST 2026-05-28)**:
  unlike autocompact (triggered when context fills up), archival is
  triggered by intent ("this proposal is flawed, archive it"). Human
  triggers primarily; model may suggest archival in <thinking>, user
  confirms. Tombstone format: "[ARCHIVED] {summary}. Status: superseded.
  Do not reference."

- **Babysitting reuses Phase III.A Step 2 infrastructure (ST 2026-05-28)**:
  Claude Code subprocess stdout streams into the existing TaskService
  event bus → SSE → ActiveTaskPanel. No new frontend or event infra.
  Adds: timeout detection, stall detection (silent stdout > N min),
  failure detection (exit code != 0 → DEAD_END candidate node).

- **Interleaved CoT deferred (ST 2026-05-28)**: provider-API interleaved
  reasoning (OpenAI Responses / DeepSeek reasoning_content / Claude
  extended thinking) NOT adopted. Prompt-level <thinking> + multi-turn
  message accumulation is the provider-agnostic equivalent and is
  sufficient for single-user research scale. Revisit only if friction
  shows Router losing reasoning state across turns.

  - **Subagent budget conservation (ST 2026-05-29, from DR Q4 Agent Contracts)**:
  fork_subagent enforces a resource conservation law: the subagent's budget
  (max_turns, token ceiling) is deducted from the parent agent's remaining
  budget, not set as an independent constant. Minimal form — NOT the full
  contract tuple C=(I,O,S,R,T,Φ,Ψ). Just: subagent budget ≤ parent remaining
  budget, enforced at fork time. Replaces the arbitrary "max_turns=3" with a
  principled ceiling. Rationale: DR Q4 shows unbounded sub-delegation is a
  top cause of runaway cost; the conservation law ∑Bᵢ ≤ B_total is the
  minimal guard.

- **DEAD_END lesson via backward trace (ST 2026-05-29, from DR Q3 CodeTracer)**:
  When babysitting detects Claude Code failure (exit != 0), the DEAD_END
  candidate node's `lesson` field is NOT the raw stdout dump. Instead, a
  single Haiku call performs backward trace: "Given this failure log, what
  was the EARLIEST decision that led to this failure? One sentence." The
  one-sentence root cause becomes the lesson. Rationale: DR Q3 CodeTracer
  shows backward-tracing from terminal error to the triggering step is more
  actionable than forward logs. Lightweight version — one Haiku call, not a
  full hierarchical trace analyzer.


## Out of Scope

- deep_research tool (Phase IV.B — consumes III.E.A subagent)
- Claude Code → Vault reflux (Phase IV.C — consumes III.E.B babysitting)
- pwsh-script-as-tool with whitelist (Phase V — high-risk, deferred)
- ingest_code_to_vault and other workflow tools (Phase IV.E — pure Python)
- Parallel / nested subagent (explicitly excluded above)
