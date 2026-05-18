# Modification Process

<key_insight>
The question is not "will this work?" — it is "will I curse myself reading this in three months?"

Code passing tests but violating taste is debt at compound interest.
</key_insight>

## Steps

<step n="1">
Read target files IN FULL. Read associated tests.

```
Read("src/oligo/core/agent.py")
Read("tests/oligo/test_agent.py")
```
</step>

<step n="2">
Grep call sites of any function planned to modify or rename:

```
Grep(pattern="compose_router_system", -n=true)
Grep(pattern="from .*agent import", -n=true)
```

If > 5 call sites → CONFIRM with user before signature change.
</step>

<step n="3">
Apply rules checklist mentally before drafting:
- [ ] New function has ≥ 3 concrete call sites? (If not → inline)
- [ ] Name describes purpose, not call path?
- [ ] Exception clauses propagate `CancelledError` + `CLIENT_GONE`?
- [ ] Every new log line has bracket prefix?
- [ ] (UI) Every CSS rule uses `var(--token)` only?
- [ ] (Pydantic) `ConfigDict(extra="forbid")` + `Field(description=...)`?
- [ ] Is this opportunistic refactoring? (If yes → STOP, ask for refactor sprint)
</step>

<step n="4">
Apply edits. Use Edit for single hunks. MultiEdit for coordinated multi-hunk in one file. Never reconstruct via Write.

```
Edit(file_path="...", old_string="...", new_string="...")
```
</step>

## Examples

<bad>
Edit("src/oligo/core/agent.py", old="...", new="...")
"Done. Should work."

(No prior Read. No call site Grep. No verification. No conformance check.)
</bad>

<good>
Read("src/oligo/core/agent.py")
Read("tests/oligo/test_agent.py")
Grep("get_prompt_composer", -n=true)              # 4 sites
Read("docs/ARCHITECTURE/PROMPT_MIDDLEWARE.md")

# rule_of_three check: 0 sites for new helper → inline

Edit(file_path="...", old_string="...", new_string="...")

Bash("./scripts/check_taste.sh src/oligo/core/agent.py")
# clean: ruff ✓, mypy ✓, pytest 23/23 ✓

# fill modification-summary template
</good>

## Success Criteria
- [ ] Target files read in full
- [ ] Test files read alongside source
- [ ] Call sites grep'd before rename / signature change
- [ ] Edits use Edit/MultiEdit (not Write reconstruction)
- [ ] Verification clean OR violations explicitly noted
- [ ] Summary template filled
- [ ] No opportunistic refactoring beyond sprint scope