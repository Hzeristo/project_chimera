# Review Process

<key_insight>
The question is not "do all tests pass?" — it is "were red lines held, and is every Partial correctly categorized?"

Green tests + violated red line = failed review.
Failed test + documented Accepted Partial = potentially successful review.
</key_insight>

## Steps

<step n="1">
Locate sprint definition + modified files.

```
Read("docs/phases/phase-{current}.md")
Bash("git diff HEAD~1 --stat")
Bash("git diff HEAD~1 -- {target_file}")
```
</step>

<step n="2">
For each dimension in reviewer prompt:

```
Read("{modified_file}")
Grep(pattern="{red_line_pattern}", path="src/")
Bash("pytest {test_path} -x -v")
Bash("python scripts/smoke_{feature}.py")  # if applicable
```
</step>

<step n="3">
Categorize per `partial-triage-guide.md`:

| Category | Definition | Action |
|---|---|---|
| **Pass** | Fully satisfied | None |
| **Accepted Partial** | Known trade-off | Append `docs/ACCEPTED_PARTIALS.md` |
| **Technical Debt** | Discovered deficiency | Append `docs/TECHNICAL_DEBT.md` |
| **Fail** | Red line violated OR existing behavior broken | Block merge, propose patch <30 lines |
</step>

<step n="4">
Verify red lines via Grep (do NOT trust agent's claim of compliance):

```
Grep(pattern="except BaseException", path="src/")
Grep(pattern="TOOL_REGISTRY\\[", path="src/")
```
</step>

<step n="5">
Emit verdict using assets/review-verdict-template.md.

After verdict approved by user:
- For Accepted Partials: append to docs/ACCEPTED_PARTIALS.md via Bash echo >>
- For Technical Debt: append to docs/TECHNICAL_DEBT.md "Open" section via Bash echo >>
- For ROADMAP.md status changes: still propose diff for user to apply manually
- For FRICTION_LOG.md status changes (OPEN → SCHEDULED → RESOLVED): propose diff
  (status changes mutate existing entries, so they're not pure append)

After append, immediately Read the file back and verify the entry is present.
If missing, alert user and stop.
</step>

## Examples

<bad>
"Sprint looks good. Tests pass. Sealed."
(No file:line evidence. No red line check. No partial categorization. No state file updates.)
</bad>

<good>
| 1.1 | router_tool_registry from ToolRegistry | Pass | `prompt_composer.py:142` | - |
| 2.4 | XML 残缺 | Accepted Partial | `tool_protocol.py:67` returns empty | Reason: graceful degradation |
| 3.2 | DENIED batch telemetry | Technical Debt | `agent.py:418` skips emit | DEBT-007 |

Red Lines:
| 不引入 lxml | Held | grep -r lxml src/ → 0 hits |

✅ Sealed. 1 Pass, 1 Accepted Partial, 1 Technical Debt filed.

Updates:
- ACCEPTED_PARTIALS.md: append "Phase III.B.2 — XML residue ignored"
- TECHNICAL_DEBT.md: append "DEBT-007: telemetry coverage gap"
</good>

## Success Criteria
- [ ] Every dimension has explicit verdict + file:line
- [ ] Every Partial categorized with reason
- [ ] Every red line verified via Grep/Bash
- [ ] Sealing decision unambiguous
- [ ] State file updates listed with proposed diffs
- [ ] No code modifications