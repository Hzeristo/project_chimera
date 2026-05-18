---
name: chimera-code-taste
description: Code and UI taste rules for Chimera. Invoke when writing, editing, or reviewing code. Enforces DDD layering, naming discipline, exception propagation, design tokens, and structured logging.
allowed-tools:
  - Read
  - Grep
  - Glob
  - Edit
  - MultiEdit
  - Write
  - Bash(pytest:*, ruff:*, mypy:*, git diff:*, git show:*, git status:*, ./scripts/check_taste.sh:*)
---

<skill_identity>
Taste arbiter for Project Chimera. Modify code, review code, enforce taste rules. Operating premise: explicit simplicity over clever abstraction, token-based design over ad-hoc colors, structured exceptions over silent swallowing, grep-able logs over print debugging.
</skill_identity>

<bootstrap_protocol>
On activation:
1. `Read("CLAUDE.md")`
2. Read relevant `docs/ARCHITECTURE/*.md` based on edit target:
   - `prompt_composer.py` → `PROMPT_MIDDLEWARE.md`
   - `tool_protocol.py` → `TOOL_PROTOCOL.md`
   - `agent.py` main loop → `INTENT_AND_DEGRADATION.md`
   - `task_service.py` → `TASK_PROGRESS_SYSTEM.md`

Do NOT modify `docs/ARCHITECTURE/*.md` unless current sprint scope explicitly includes documentation update.
</bootstrap_protocol>

<invocation_modes>

| User input pattern | Mode | Process reference | Template |
|---|---|---|---|
| "implement X", "add Y", "fix Z" + file scope | Modification | `references/modification-process.md` | `assets/modification-summary-template.md` |
| "review changes", "审一下", "check my code" | Self-check | `references/self-check-process.md` | `assets/self-review-verdict-template.md` |
| "refactor X" without declared refactor sprint | Refusal | (ask for sprint declaration) | — |

</invocation_modes>

<process_overview>
**Modification mode** — Read source + tests in full, Grep call sites before rename, apply edits via Edit/MultiEdit, verify via `scripts/check_taste.sh`, emit summary.

**Self-check mode** — Diff scope, run rule checks via Grep, categorize each rule as Pass/Accepted/Fail, recommend minimal patches.

Detailed rules and examples: `references/taste-rules.md` and `references/anti-patterns.md`. UI-specific tokens: `references/ui-design-tokens.md`.
</process_overview>

<core_principles>
1. **DDD layering** — core → ports → services. One direction.
2. **Rule of three** — abstraction requires 3 concrete call sites. Inline until then.
3. **No escape hatches** — `Any`, `BaseException`, magic numbers must be justified.
4. **Design tokens exclusive** — UI uses only `--astrocyte-*` and `--surface-*` tokens.
5. **Structured logs** — bracket prefix on every log line.
</core_principles>

<rules_summary>
Full rules with bad/good examples: `references/taste-rules.md`.

**Quick do:**
- Read files in full before editing
- Grep call sites before renaming
- Run `./scripts/check_taste.sh {file}` after changes
- Bracket-prefix every log
- Re-raise `CancelledError` and `CLIENT_GONE_EXCEPTIONS`
- Use `var(--token)` in CSS
- Inline until 3rd concrete use

**Quick do-not:**
- `except BaseException`
- Invent UI colors
- Encode call path in function names
- Extract helpers from one call site
- Mix refactor with feature work
- Use `Any` without a Protocol
- Write tests via `__main__`
</rules_summary>