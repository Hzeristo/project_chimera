> **Format scope:** The schema below applies strictly to NEW entries.
> Pre-existing entries may use legacy field labels (`想做的事`/`我想做的事情`,
> `实际怎么做的`/`我实际怎么做的`, `摩擦成本`) and may omit the status tag
> on the heading or the `根因` field. Do not retroactively rewrite legacy
> entries; tolerate variance in reads.

### Entry {N} [Status: OPEN | SCHEDULED | RESOLVED | WONTFIX]
- 时间: YYYY.MM.DD HH:MM
- 想做: {clear_goal}
- 实际: {actual_workflow}
- 根因: {root_cause}
- 成本: {coffee | minutes | 气的}
- 理想: {one_line_desired_behavior}

(If status is SCHEDULED or RESOLVED, append:)
**Resolution:** Phase {X.Y} / Sprint {N} — {short_summary}  