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
- prompt_composer.py → docs/ARCHITECTURE/PROMPT_MIDDLEWARE.md
- tool_protocol.py → docs/ARCHITECTURE/TOOL_PROTOCOL.md
- agent.py main loop → docs/ARCHITECTURE/INTENT_AND_DEGRADATION.md
- task_service.py → docs/ARCHITECTURE/TASK_PROGRESS_SYSTEM.md

Verify Python env path is declared in CLAUDE.md (else STOP and ask user).
</bootstrap_protocol>

<invocation_modes>

| User input pattern | Mode | Process | Template |
|---|---|---|---|
| "execute sprint {N}", "execute batch {phase}", "run FC.{N..M}" | batch_execution | references/batch-execution-process.md | assets/modification-summary-template.md |

</invocation_modes>

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
