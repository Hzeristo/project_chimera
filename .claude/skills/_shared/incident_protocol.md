# Incident Protocol (shared)

Single source for chimera-code-taste and chimera-sprint-discipline. Edit here
only — both skills point at this file instead of carrying their own copy.

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
