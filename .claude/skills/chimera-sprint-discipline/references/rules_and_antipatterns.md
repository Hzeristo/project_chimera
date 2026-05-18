# Rules and Anti-Patterns

## Rules

<rule id="sprint_size">
**Statement:** One sprint touches ≤ 3 files and adds ≤ 50 new lines.

**Bad:**
Sprint touching `agent.py` + `prompt_composer.py` + `tool_protocol.py` + `server.py` + 2 frontend files (~200 lines).

**Good:**
Sprint FC.1: `schemas.py` + `vault_tools.py` + `agent.py` (~65 lines, 3 files). Sibling sprints handle the rest.
</rule>

<rule id="red_line_format">
**Statement:** Every sprint prompt has ≥ 3 explicit 红线 prohibitions.

**Bad:**
"Sprint: Add web_search telemetry."
(No 红线 — agent may refactor unrelated code.)

**Good:**
红线:
- ❌ 不修改 web_search.py 执行逻辑
- ❌ 不引入新 SSE 事件类型
- ❌ 不改变 ToolRegistry 对外接口
</rule>

<rule id="partial_triage">
**Statement:** Every Partial categorized: Accepted / Technical Debt / Fail.

**Bad:**
"3 Partials found. Spinning up follow-up sprint to fix all 3."

**Good:**
- Partial 1: Accepted (length budget trade-off)
- Partial 2: Accepted (depends on live LLM)
- Partial 3: Technical Debt → DEBT-008 → debt week
</rule>

<rule id="phase_completion_signal">
**Statement:** Phase done = friction resolved. Not phase done = sprint count reached.

**Bad:**
"Phase II.B: 3 sprints ran, all green. Done." (User still uses terminal — friction unchanged.)

**Good:**
"Phase II.B: 3 sprints ran. Use Week confirms terminal usage dropped to zero. Sealed."
</rule>

## Anti-Patterns

<anti_pattern id="roadmap_driven">
Sprints to complete a roadmap rather than resolve friction. **Reject** without friction reference.
</anti_pattern>

<anti_pattern id="opportunistic_refactor">
Mid-sprint scope creep ("while I'm here"). **Hard-stop** at declared file list.
</anti_pattern>

<anti_pattern id="all_partials_fix">
Treating every Partial as defect. **Triage** explicitly.
</anti_pattern>

<anti_pattern id="ceremony_theater">
Full sprint structure for typo fix. **Bypass** ceremony for trivial changes.
</anti_pattern>