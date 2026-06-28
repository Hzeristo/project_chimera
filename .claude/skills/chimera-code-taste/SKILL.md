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

| Operation | Recommended | Acceptable | Wasteful |
|---|---|---|---|
| Reading source for context | Sonnet | Opus | Haiku (insufficient) |
| Editing code (Edit, replace_all) | Sonnet | Opus (large refactor) | Haiku (insufficient) |
| Running pytest/ruff/mypy in subagent | Haiku | Sonnet | Opus (5x cost overrun) |

On activation, if current model is Opus:
  Output before any other work:

    Cost note: batch_execution is execution-shaped, not reasoning-shaped.
    Sonnet is sufficient for sprint execution constrained by red lines and
    explicit task scope. Estimated cost overrun on Opus: 3-5x.
    Switch with /model to Sonnet, OR confirm to continue with Opus.

  Wait for confirmation.

Subagent verification tasks should use Haiku via Agent tool with
{model: "haiku"} parameter (or a current Haiku model id).
</expected_model>

<subagent_routing>
Spawn subagents (Agent tool, general-purpose, model: Haiku) for:
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
<!-- SYNC: duplicated in chimera-sprint-discipline, manual sync needed -->
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
<!-- SYNC: duplicated in chimera-sprint-discipline, manual sync needed -->
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
