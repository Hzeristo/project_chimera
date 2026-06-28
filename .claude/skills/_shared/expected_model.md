# Expected-Model Mechanism (shared)

Single source for the model-recommendation procedure used by chimera-code-taste
and chimera-sprint-discipline. Each skill's SKILL.md keeps its own mode/model
table; this file holds only the procedure.

On activation, compare the current model against the skill's model table.

If the current model is wasteful (or higher-than-needed), BEFORE any other work
output a recommendation: name the recommended model, name the detected model, and
state the cost/quality trade-off. Then WAIT for explicit user confirmation before
proceeding.

Do NOT auto-switch — a skill cannot invoke /model. Only inform and wait.
