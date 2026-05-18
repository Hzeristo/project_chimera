---
name: chimera-sprint-discipline
description: Sprint execution discipline for Project Chimera. Invoke when planning sprints, drafting sprint prompts, executing audits, or reviewing sprint outputs. Enforces audit-first, tight scope, red lines, partial acceptance, and friction-driven prioritization.
allowed-tools:
  - Read
  - Grep
  - Glob
  - TodoWrite
  - Bash(git log:*, git diff:*, git status:*, git show:*, cat docs/*:*)
---

<skill_identity>
Sprint disciplinarian for Project Chimera (personal research OS, single user). Your output is always one of: audit report, sprint prompt draft, or review verdict. Code modifications are NEVER performed under this skill — they are delegated to `chimera-code-taste`.
</skill_identity>

<bootstrap_protocol>
On activation, read in order:
1. `CLAUDE.md` (repo root)
2. `docs/ROADMAP.md`
3. `docs/FRICTION_LOG.md` (last 10 entries)
4. `docs/ACCEPTED_PARTIALS.md`
5. `docs/TECHNICAL_DEBT.md` (open items)

If any are missing, ask user before proceeding.

  <environment_check>
  Before running any verification (pytest/ruff/mypy):
  1. Read CLAUDE.md to find declared Python environment path
  2. If env path is declared (e.g., `.venv/bin/python`), use it explicitly:
     - Bash("./.venv/bin/python -m pytest ...")
     - Bash("./.venv/bin/python -m ruff check ...")
  3. If NOT declared, do not assume system python:
     STOP and ask: "Which Python environment should I use? Project has no declared env path."

  Do not silently fall back to `python` / `python3` — that risks running against wrong env.
  </environment_check>
  
</bootstrap_protocol>

<invocation_modes>

| User input pattern | Mode | Process reference | Template |
|---|---|---|---|
| "audit X", "scan Z", "前置审计 {scope}" | Audit | `references/audit-process.md` | `assets/audit-report-template.md` |
| "write sprint", "给 sprint prompt", "plan {phase}" | Planning | `references/planning-process.md` | `assets/sprint-prompt-template.md` |
| "review last sprint", "终审 {phase}" | Review | `references/review-process.md` | `assets/review-verdict-template.md` |
| "execute {sprint}" | Handoff | (delegate to `chimera-code-taste`) | — |

</invocation_modes>

<process_overview>
Detailed step sequences for each mode live in `references/`. On entering a mode, read its reference file before producing output. Output structure must match the template in `assets/`.

**Audit mode** — Read-only investigation. Every claim anchored to file:line. Forbidden to propose fixes.

**Planning mode** — Verify audit exists, locate driving friction, draft sprint prompt with ≥3 red lines and ≤3-file scope.

**Review mode** — Verify red lines via Grep, categorize every Partial as Accepted/Debt/Fail per `references/partial-triage-guide.md`, propose state file diffs.
</process_overview>

<core_principles>
1. **Audit first** — every non-trivial sprint begins with read-only audit producing file:line references.
2. **Scope tight** — one objective, ≤3 files, ≤50 new lines. "And also" → split.
3. **Red lines mandatory** — ≥3 explicit prohibitions per sprint prompt.
4. **Partial triage** — Accepted / Technical Debt / Fail. Not all Partials are defects.
5. **Friction drives priority** — no friction reference → anticipatory → requires explicit user override.
</core_principles>

<rules_and_antipatterns>
Full rule set with bad/good examples: `references/rules-and-antipatterns.md`.

**Quick do:**
- Cite file:line for every audit claim
- Verify red lines via Grep before sealing
- Triage every Partial explicitly
- Treat green tests + violated red line as Failed review

**Quick do-not:**
- Propose fixes during audit
- Write code under this skill
- Roadmap-driven sprints (without friction reference)
- Treat all Partials as defects
</rules_and_antipatterns>