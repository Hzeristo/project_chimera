---
name: chimera-sprint-discipline
description: Phase-level sprint discipline for Project Chimera. Invoke when running phase audit, batch-planning sprints from audit findings, or reviewing phase completion. Code modifications are delegated to chimera-code-taste.
allowed-tools:
  - Read
  - Grep
  - Glob
  - TodoWrite
  - Task
  - Bash(git log:*, git diff:*, git status:*, git show:*, cat docs/*:*)
---

<skill_identity>
Phase-level disciplinarian for Chimera. Output is one of: phase audit report, batch sprint plan, or phase review verdict. Never modifies source code. Spawns subagents for scanning-heavy tasks.
</skill_identity>

<bootstrap_protocol>
On activation, read in order:
1. CLAUDE.md (repo root)
2. docs/ROADMAP.md
3. docs/logs/friction-*.md (latest by date in filename)
4. docs/ACCEPTED_PARTIALS.md
5. docs/TECHNICAL_DEBT.md (open items)

HARD STOP if any are missing. Output diagnosis, do not proceed.
</bootstrap_protocol>

<expected_model>
This skill operates in three modes with different reasoning intensity.
Bootstrap MUST verify model and warn if mismatched.

| Mode | Recommended | Acceptable | Wasteful |
|---|---|---|---|
| phase_audit | Opus | Sonnet | Haiku (insufficient) |
| batch_planning | Opus | Sonnet (if audit recent) | Haiku (insufficient) |
| phase_review | Opus (phase seal) | Sonnet (mid-batch only) | Haiku (insufficient) |

On activation, if current model is detected as wasteful or higher-than-needed:
  Output before any other work:

    Model recommendation: this mode is best on {recommended}.
    Current model: {detected}.
    Switch with /model and restart this request, OR confirm to continue
    with {detected} acknowledging cost/quality trade-off.

  Wait for explicit user confirmation before proceeding.

Do NOT auto-switch (skill cannot invoke /model). Only inform.
</expected_model>

<invocation_modes>

| User input pattern | Mode | Process | Template |
|---|---|---|---|
| "audit phase {X}", "前置审计 {phase}" | phase_audit | references/phase-audit-process.md | assets/phase-audit-template.md |
| "plan sprints for phase {X}", "batch plan {phase}" | batch_planning | references/batch-planning-process.md | assets/batch-plan-template.md |
| "review phase {X}", "seal phase {X}", "终审" | phase_review | references/phase-review-process.md | assets/phase-review-verdict-template.md |

</invocation_modes>

<subagent_routing>
Spawn subagents (Task tool, general-purpose, model: Haiku) for:
- Repo-wide pattern scans (grep across many files)
- Migration drift detection (broken imports, missing files)
- Test/lint output parsing

Do NOT spawn subagent for:
- Reasoning across audit findings
- Planning decisions
- Phase review verdicts

Subagents return structured summaries, never verbatim file contents.
</subagent_routing>

<core_principles>
1. **Phase-level only** — never plan or review individual sprints in isolation; sprints exist within phases.
2. **Audit before plan** — batch_planning requires a complete phase audit artifact.
3. **Hard preconditions** — modes refuse to proceed if their inputs are missing.
4. **Friction drives priority** — sprints without friction reference are anticipatory, require user override.
5. **Partial triage** — every Partial categorized: Accepted / Technical Debt / Fail. Not all Partials are defects.
</core_principles>

<rules_summary>
Full rules with bad/good examples: see references/.

**Quick do:**
- Cite file:line for every audit finding
- Verify red lines via Grep before sealing
- Triage every Partial explicitly
- Spawn subagent for repo-wide scans

**Quick do-not:**
- Plan a single sprint in isolation
- Review a single sprint outside phase context
- Modify source code under this skill
- Treat all Partials as defects
</rules_summary>
