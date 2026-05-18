# Planning Process

<key_insight>
The question is not "what should we build?" — it is "what friction does this resolve, and what is the smallest change that resolves it?"

A sprint without a friction reference is anticipatory. Anticipatory sprints require explicit user override.
</key_insight>

## Steps

<step n="1">
Verify an audit exists for this sprint's scope.

```
Read("docs/phases/phase-{current}.md")
Glob("docs/audits/*{scope}*")
```

If no audit, ask: "Run sprint_audit first, or proceed (acceptable for trivial sprints)?"
</step>

<step n="2">
Locate driving friction.

```
Read("docs/FRICTION_LOG.md")
Grep(pattern="OPEN|SCHEDULED", path="docs/FRICTION_LOG.md", -n=true)
```

Find entry this sprint addresses. Record `{friction_id}`. If none → mark Anticipatory, require justification.
</step>

<step n="3">
Single objective. ONE sentence beginning with a verb. If "and also" appears → STOP, split.
</step>

<step n="4">
Estimate scope:

```
Bash("wc -l src/oligo/core/agent.py")
```

If estimate exceeds 3 files OR 50 new lines OR 5 distinct tasks → split.
</step>

<step n="5">
Fill `assets/sprint-prompt-template.md`. Output as single fenced text block. Do not execute. Await approval.
</step>

## Examples

<bad>
"Let me write a sprint to fix the tool calling and also clean up the prompts and update the docs and add some tests."

(Multi-objective. No friction reference. No red lines. No file scope.)
</bad>

<good>
See filled template in `assets/sprint-prompt-template.md` for FC.1 example.
</good>

## Disambiguation: phase planning vs sprint planning

If user asks for "all sprints" / "the phase plan" / "complete sprint set":
- This is **phase orchestration**, not sprint planning
- Phase orchestration belongs in `docs/phases/{phase}.md` (user-authored)
- Respond:
  "Skill discipline plans one sprint at a time, with audit-execute-review feedback between each. To plan all of {phase}, the phase doc at `docs/phases/{phase}.md` already serves that role.
  Options: (a) plan only the next sprint, (b) review/refine the phase doc, (c) override and batch-plan with explicit acknowledgment that audit-execute-review feedback is sacrificed.
  Which?"


## Success Criteria
- [ ] Single objective in one sentence beginning with verb
- [ ] Friction reference cited (or anticipatory justification)
- [ ] ≥ 3 red line prohibitions
- [ ] ≤ 5 tasks with file paths and line estimates
- [ ] Total ≤ 3 files modified, ≤ 50 new lines
- [ ] Output is single fenced text block
- [ ] No code modifications attempted
- [ ] User approval requested before handoff