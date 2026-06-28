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
Spawn subagents (Agent tool, general-purpose, model: Haiku) for:
- Repo-wide pattern scans (Select-String / Grep across many files)
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

<execution_environment>
<!-- SYNC: duplicated in chimera-code-taste, manual sync needed -->
Project Chimera development host: Windows.
Tool invocations use the PowerShell tool (pwsh 7+), NOT Bash.

PowerShell-native idioms to use:
  - File listing: Get-ChildItem path/ -Recurse -Filter *.py
  - Existence check: Test-Path path/
  - Pattern search: Select-String -Path 'path/*.py' -Pattern 'foo' -Recurse
  - File content: Get-Content path/file.md
  - Append to file: "text" | Out-File -Append -Encoding utf8 path/file.md
  - Remove file: Remove-Item path/file.md -Force
  - Conditional: $x ? "yes" : "no" (pwsh 7+ ternary)
  - Null coalescing: $x ?? "default" (pwsh 7+)

Cross-platform commands that work as-is:
  - git log/diff/status/show/add/commit
  - python -m pytest/ruff/mypy
  - cargo test/build
  - npm run check / pnpm check

Do NOT use Unix-only syntax:
  - find (use Get-ChildItem -Recurse)
  - grep (use Select-String)
  - rm (use Remove-Item)
  - cat (use Get-Content — alias exists but not in tool globs)
  - test -f (use Test-Path)
  - echo "x" >> file (use Out-File -Append)
  - [ -d path ] (use Test-Path -PathType Container)
  - wc -l (use (Get-Content file).Count)

Construct correct syntax on first attempt. No POSIX-then-retry pattern.
</execution_environment>

<incident_protocol>
<!-- SYNC: duplicated in chimera-code-taste, manual sync needed -->
An incident is a clear-cut code defect (regression, crash, parse failure)
that is immediately diagnosable and fixable. It is NOT a friction (workflow
pain) nor tracked debt (known deferred work).

Incident handling bypasses phase/sprint machinery:
1. Reproduce + locate root cause (cite file:line)
2. Apply minimal hotfix
3. Run the specific test/smoke that reproduces it; add a regression test
4. Commit as: fix({scope}): {one-line} — hotfix, no sprint
5. Log to docs/incidents/{YYYY-MM-DD}-{slug}.md: root cause + fix, 2 lines

Do NOT:
- Log incidents to FRICTION_LOG (pollutes workflow-pain signal)
- Defer incidents to debt week (they need immediate fix)
- Run full audit/plan cycle (root cause is already known)

Escalation: if the SAME class of incident recurs ≥3 times, promote it to a
FRICTION entry — recurrence indicates a workflow/architecture problem, not
an isolated bug.
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
