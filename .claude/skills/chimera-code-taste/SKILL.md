---
name: chimera-code-taste
description: Code and UI taste enforcement during sprint execution within a batch. Invoke when executing approved sprints batch-style. Enforces DDD layering, naming, exception propagation, design tokens, structured logging.
allowed-tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Write
  - Agent
  - PowerShell(pytest:*, ruff:*, mypy:*, git diff:*, git show:*, git status:*, .claude/skills/chimera-code-taste/scripts/check_taste.ps1:*)
---

<skill_identity>
Taste arbiter executing pre-approved sprints in batch. Modifies code, runs verification, returns structured summaries. Each sprint commits independently. Halts batch on red-line violation.
</skill_identity>

<bootstrap_protocol>
On activation, read CLAUDE.md and the architecture doc(s) relevant to current sprint target:
- prompt_composer.py → docs/architecture/PROMPT_MIDDLEWARE.md
- tool_protocol.py → docs/architecture/TOOL_PROTOCOL.md
- agent.py main loop → docs/architecture/INTENT_AND_DEGRADATION.md
- task_service.py → docs/architecture/TASK_PROGRESS_SYSTEM.md

Verify Python env path is declared in CLAUDE.md (else STOP and ask user).
</bootstrap_protocol>

<invocation_modes>

| User input pattern | Mode | Process | Template |
|---|---|---|---|
| "execute sprint {N}", "execute batch {phase}", "run FC.{N..M}" | batch_execution | references/batch_execution_process.md | assets/modification-summary-template.md |

</invocation_modes>

<expected_model>
This skill operates batch_execution mode. Sprint prompts are pre-approved with
red lines and file scope, so execution does not need cross-cutting reasoning.
Recommendation procedure (detect → inform → wait, never auto-switch):
see ../_shared/expected_model.md.

| Operation | Recommended | Acceptable | Wasteful |
|---|---|---|---|
| Reading source for context | Sonnet | Opus | Haiku (insufficient) |
| Editing code (Edit, replace_all) | Sonnet | Opus (large refactor) | Haiku (insufficient) |
| Running pytest/ruff/mypy in subagent | Haiku | Sonnet | Opus (5x cost overrun) |

If the current model is Opus, before any other work output the cost note
(batch_execution is execution-shaped, not reasoning-shaped; Sonnet suffices;
~3-5x overrun on Opus) and wait for confirmation.

Subagent verification tasks should use Haiku via Agent tool with
{model: "haiku"} parameter (or a current Haiku model id).
</expected_model>

<subagent_routing>
Generic delegation policy (Haiku for scans, structured returns, prose is never
the verdict): see ../_shared/subagent_routing.md. Skill-specific:

Spawn subagents (Haiku) for:
- Running check_taste.ps1 and parsing output
- Running pytest suite and summarizing failures
- Cross-file rule violation scanning

Do NOT spawn subagent for:
- Editing code (Edit/Write must be main session)
- Reading source files for editing context
- Self-check rule application (the rule application IS the reasoning)

Subagent return contract (verification tasks): the subagent MUST return the
verbatim last 10 lines of check_taste.ps1 output AND the script's exit code.
The main session decides pass/fail from the **exit code alone** (0 = pass,
non-zero = fail) — never from the subagent's prose. A paraphrased summary may
accompany the tail for context, but it is NOT authoritative; the exit code is.
A missing or non-integer exit code is treated as FAIL (halt), never as pass.
For non-verification scans, subagents return file:line of violations.
</subagent_routing>

<core_principles>
1. **DDD layering** — core → ports → services. One direction.
2. **Rule of three** — abstraction requires 3 concrete call sites.
3. **No escape hatches** — Any, BaseException, magic numbers must be justified.
4. **Design tokens exclusive** — UI uses only --astrocyte-* and --surface-* tokens.
5. **Structured logs** — bracket prefix on every log line.
6. **Halt on red line** — first sprint in batch that violates a red line stops the entire batch.
</core_principles>

<execution_environment>
Host is Windows — use the PowerShell tool (pwsh 7+), NOT Bash. Full idiom list
and forbidden POSIX syntax: see ../_shared/execution_environment.md (single
source shared with chimera-sprint-discipline).
</execution_environment>

<incident_protocol>
Incidents (clear-cut code defects: regression, crash, parse failure) bypass
phase/sprint machinery — reproduce + root-cause, minimal hotfix, regression
test, commit `fix({scope}): …`, log to docs/incidents/. Full protocol (incl.
the ≥3-recurrence → FRICTION escalation): see ../_shared/incident_protocol.md
(single source shared with chimera-sprint-discipline).
</incident_protocol>

<rules_summary>
Full rules with bad/good examples: references/taste_rules.md, references/anti_patterns.md, references/ui_design_tokens.md.

**Quick do:**
- Read files in full before editing
- Grep call sites before renaming
- Run check_taste.ps1 after changes
- Bracket-prefix every log
- Re-raise CancelledError + CLIENT_GONE_EXCEPTIONS
- Use var(--token) in CSS

**Quick do-not:**
- except BaseException
- Invent UI colors
- Encode call path in function names
- Extract helpers from one call site
- Mix refactor with feature work
- Use Any without a Protocol
</rules_summary>
