# Subagent Routing (shared policy)

Single source for the generic delegation policy used by chimera-code-taste and
chimera-sprint-discipline. Each skill's SKILL.md keeps its own specific
delegate / do-not-delegate lists (and code-taste keeps the exit-code return
contract); this file holds only the common rules.

Spawn subagents via the Agent tool (general-purpose, model: Haiku) for
mechanical, high-volume, read-only work: repo-wide pattern scans, test/lint
output parsing, cross-file violation scanning.

Never spawn a subagent for reasoning that IS the work: planning decisions,
audit/review verdicts, rule application, editing code, or reading source for
editing context.

Subagents return compact STRUCTURED results (file:line of violations, or the
contracted verification tail), never verbatim file contents. A subagent's prose
is evidence for the main session, never the verdict.
