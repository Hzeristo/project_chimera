---
name: chimera-code-taste
description: Code and UI taste enforcement during sprint execution within a batch. Invoke when executing approved sprints batch-style. Enforces DDD layering, naming, exception propagation, design tokens, structured logging.
allowed-tools:
  - Read
  - Grep
  - Glob
  - Edit
  - MultiEdit
  - Write
  - Task
  - Bash(pytest:*, ruff:*, mypy:*, git diff:*, git show:*, git status:*, ./scripts/check_taste.sh:*)
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
| "execute sprint {N}", "execute batch {phase}", "run FC.{N..M}" | batch_execution | references/batch-execution-process.md | assets/modification-summary-template.md |

</invocation_modes>

<expected_model>
This skill operates batch_execution mode. Sprint prompts are pre-approved with
red lines and file scope, so execution does not need cross-cutting reasoning.

| Operation | Recommended | Acceptable | Wasteful |
|---|---|---|---|
| Reading source for context | Sonnet | Opus | Haiku (insufficient) |
| Editing code (Edit/MultiEdit) | Sonnet | Opus (large refactor) | Haiku (insufficient) |
| Running pytest/ruff/mypy in subagent | Haiku | Sonnet | Opus (5x cost overrun) |

On activation, if current model is Opus:
  Output before any other work:

    Cost note: batch_execution is execution-shaped, not reasoning-shaped.
    Sonnet is sufficient for sprint execution constrained by red lines and
    explicit task scope. Estimated cost overrun on Opus: 3-5x.
    Switch with /model to Sonnet, OR confirm to continue with Opus.

  Wait for confirmation.

Subagent verification tasks should use Haiku via Task tool with
{model: "claude-haiku-4-5"} parameter (or whichever Haiku version is current).
</expected_model>


<subagent_routing>
Spawn subagents (Task tool, general-purpose, model: Haiku) for:
- Running check_taste.sh and parsing output
- Running pytest suite and summarizing failures
- Cross-file rule violation scanning

Do NOT spawn subagent for:
- Editing code (Edit/MultiEdit/Write must be main session)
- Reading source files for editing context
- Self-check rule application (the rule application IS the reasoning)

Subagents return only: pass/fail, failure summaries, file:line of violations.
</subagent_routing>

<core_principles>
1. **DDD layering** — core → ports → services. One direction.
2. **Rule of three** — abstraction requires 3 concrete call sites.
3. **No escape hatches** — Any, BaseException, magic numbers must be justified.
4. **Design tokens exclusive** — UI uses only --astrocyte-* and --surface-* tokens.
5. **Structured logs** — bracket prefix on every log line.
6. **Halt on red line** — first sprint in batch that violates a red line stops the entire batch.
</core_principles>

<rules_summary>
Full rules with bad/good examples: references/taste-rules.md, references/anti-patterns.md, references/ui-design-tokens.md.

**Quick do:**
- Read files in full before editing
- Grep call sites before renaming
- Run check_taste.sh after changes
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
