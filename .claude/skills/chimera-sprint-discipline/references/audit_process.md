# Audit Process

<key_insight>
The question is not "is the code right?" — it is "do I understand what the code actually does, with file:line precision?"

An audit fails when it produces guesses. An audit succeeds when every claim is anchored to a verifiable line.
</key_insight>

## Steps

<step n="1">
Identify audit scope from user request. Extract: (a) target files/modules, (b) specific questions.

If scope is vague, ask ONE clarifying question. Do not assume.

```
Bash("git log --oneline -20")
Bash("cat docs/ROADMAP.md")
```
</step>

<step n="2">
Read every in-scope file IN FULL.

```
Read("src/oligo/core/agent.py")
Read("src/oligo/tools/registry.py")
Read("tests/oligo/test_agent.py")
```
</step>

<step n="3">
Cross-reference patterns:

```
Grep(pattern="def {function_name}", output_mode="content", -n=true)
Grep(pattern="from .* import {symbol}", -n=true)
Grep(pattern="TOOL_REGISTRY\\[", -n=true)
```
</step>

<step n="4">
For every finding, capture:
- **What** (one sentence)
- **Where** (`{path}:{line}`)
- **Risk** (Low / Med / High)

Do NOT propose fixes. Do NOT draft sprint prompts.
</step>

<step n="5">
Emit output using `assets/audit-report-template.md`.

Do NOT say "Audit complete" if any question lacks file:line.
</step>

## Examples

<bad>
Read("agent.py")
"Looking at this, I think the prompt composer is invoked around line 340."
"This should be safe to refactor."
</bad>

<good>
Read("src/oligo/core/agent.py")
Grep("get_prompt_composer", -n=true)         # 4 sites
Read("tests/oligo/test_prompt_composer.py")

| Q1 | Where injected? | At `agent.py:347-351`, via `get_prompt_composer().compose(stage="router")` | `agent.py:347` | Low |
| Q2 | Stateful? | No, stateless after `_register_default_components`. | `prompt_composer.py:114-198` | Low |
</good>

## Success Criteria
- [ ] Every audit question has explicit answer
- [ ] Every answer cites at least one file:line
- [ ] No fix proposals included
- [ ] No code modifications attempted
- [ ] Output matches audit-report-template.md
- [ ] Next-step options offered to user