---
name: chimera-sprint-discipline
description: Phase-level sprint discipline for Project Chimera. Invoke when running phase audit, batch-planning sprints from audit findings, or reviewing phase completion. Code modifications are delegated to chimera-code-taste.
allowed-tools:
  - Read
  - Grep
  - Glob
  - TaskCreate
  - Agent
  - PowerShell(git log:*, git diff:*, git status:*, git show:*, Get-Content docs/*:*)
  - Edit  # required for status field flips
  - PowerShell(*Out-File -Append*docs/ACCEPTED_PARTIALS.md*)
  - PowerShell(*Out-File -Append*docs/TECHNICAL_DEBT.md*)
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

| Mode | Recommended | Acceptable | Wasteful |
|---|---|---|---|
| phase_audit | Opus | Sonnet | Haiku (insufficient) |
| batch_planning | Opus | Sonnet (if audit recent) | Haiku (insufficient) |
| phase_review | Opus (phase seal) | Sonnet (mid-batch only) | Haiku (insufficient) |

On activation, if the current model is Wasteful (or higher-than-needed) for the
selected mode, follow the recommendation procedure BEFORE any other work
(detect → inform → wait, never auto-switch): see ../_shared/expected_model.md.
</expected_model>

<invocation_modes>

| User input pattern | Mode | Process | Template |
|---|---|---|---|
| "audit phase {X}", "前置审计 {phase}" | phase_audit | references/phase-audit-process.md | assets/phase-audit-template.md |
| "plan sprints for phase {X}", "batch plan {phase}" | batch_planning | references/batch-planning-process.md | assets/batch-plan-template.md |
| "review phase {X}", "seal phase {X}", "终审" | phase_review | references/phase-review-process.md | assets/phase-review-verdict-template.md |

</invocation_modes>

<state_write_authority>
This skill has restricted write authority over state files in phase_review mode only:

AUTO-APPLY (no user approval required):
- Append to docs/ACCEPTED_PARTIALS.md
- Append to docs/TECHNICAL_DEBT.md
- Flip friction status OPEN/SCHEDULED → RESOLVED for frictions this phase resolved

PROPOSE-DIFF (user approval required):
- docs/ROADMAP.md
- friction-*.md status changes outside phase-resolution
- Any conflicting modifications to existing entries

NEVER WRITE in any mode:
- CLAUDE.md
- Skill files themselves
- Source code (handed to chimera-code-taste)
- Architecture docs (chimera-code-taste only, in sprint scope)

This authority exists ONLY in phase_review mode. phase_audit and
batch_planning modes remain read-only as before.
</state_write_authority>

<subagent_routing>
Generic delegation policy (Haiku for scans, structured returns, prose is never
the verdict): see ../_shared/subagent_routing.md. Skill-specific:

Spawn subagents (Haiku) for:
- Repo-wide pattern scans (Select-String / Grep across many files)
- Migration drift detection (broken imports, missing files)
- Test/lint output parsing

Do NOT spawn subagent for:
- Reasoning across audit findings
- Planning decisions
- Phase review verdicts
</subagent_routing>

<core_principles>
1. **Phase-level only** — never plan or review individual sprints in isolation; sprints exist within phases.
2. **Audit before plan** — batch_planning requires a complete phase audit artifact.
3. **Hard preconditions** — modes refuse to proceed if their inputs are missing.
4. **Friction drives priority** — sprints without friction reference are anticipatory, require user override.
5. **Partial triage** — every Partial categorized: Accepted / Technical Debt / Fail. Not all Partials are defects.
</core_principles>

<execution_environment>
Host is Windows — use the PowerShell tool (pwsh 7+), NOT Bash. Full idiom list
and forbidden POSIX syntax: see ../_shared/execution_environment.md (single
source shared with chimera-code-taste).
</execution_environment>

<incident_protocol>
Incidents (clear-cut code defects: regression, crash, parse failure) bypass
phase/sprint machinery — reproduce + root-cause, minimal hotfix, regression
test, commit `fix({scope}): …`, log to docs/incidents/. Full protocol (incl.
the ≥3-recurrence → FRICTION escalation): see ../_shared/incident_protocol.md
(single source shared with chimera-code-taste).
</incident_protocol>

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
