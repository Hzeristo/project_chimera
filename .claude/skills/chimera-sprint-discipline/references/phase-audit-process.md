# Phase Audit Process

<key_insight>
Phase audit produces the file:line evidence base that all sprints in the phase
will plan against. An audit fails when it produces guesses. An audit succeeds
when every claim is anchored.
</key_insight>

## Hard Preconditions
1. Phase doc exists at docs/phases/phase-{X}.md
2. Phase doc has Mission + Driving frictions + Sprint name list
3. Phase doc does NOT have detailed per-sprint task lists (those are batch_planning's job)

## Steps

<step n="1">
Read phase doc. Identify sprint names + each sprint's one-line goal.
</step>

<step n="2">
For each sprint goal, derive 1-3 audit questions. Output Q list before reading code.
Ask user for Q list approval (one round of clarification permitted).
</step>

<step n="3">
Read every in-scope file in full. Spawn subagent for repo-wide pattern scans.
</step>

<step n="4">
For each Q, write answer with file:line evidence + Risk (Low/Med/High).
</step>

<step n="5">
Identify cross-findings (audit revelations not directly answering a Q).
Flag, do not propose fixes.
</step>

<step n="6">
Output using assets/phase-audit-template.md. Write to docs/audits/phase-{X}.md.
</step>

## Success Criteria
- [ ] Every Q has file:line answer
- [ ] Cross-findings flagged separately from Q answers
- [ ] No fix proposals
- [ ] Output committed to docs/audits/phase-{X}.md
