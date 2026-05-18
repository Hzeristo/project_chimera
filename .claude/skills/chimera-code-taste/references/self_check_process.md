# Self-Check Process

<key_insight>
The question is not "does the test pass?" — it is "does each change conform to taste rules, and are violations either accepted or fixed before merge?"
</key_insight>

## Steps

<step n="1">
Identify scope:

```
Bash("git diff HEAD --stat")
Bash("git diff HEAD -- {target_path}")
```
</step>

<step n="2">
For each modified file, read diff + current state:

```
Bash("git diff HEAD -- src/oligo/core/agent.py")
Read("src/oligo/core/agent.py")
```
</step>

<step n="3">
Run rule checks via Grep:

```
# function_naming
Grep(pattern="def [a-z_]{40,}", path="src/")

# rule_of_three (manual count)
Grep(pattern="{new_helper_name}", -n=true)

# exception_handling
Grep(pattern="except BaseException", path="src/")              # forbidden
Grep(pattern="except CancelledError|CLIENT_GONE", path="src/") # presence

# logging_format
Grep(pattern="logger\\.(info|warning|error)\\(\"[^\\[]", path="src/")
```
</step>

<step n="4">
For new CSS (if applicable):

```
Grep(pattern="#[0-9a-fA-F]{3,8}", path="astrocyte/src/", glob="*.css")
Grep(pattern="padding: \\d+px", path="astrocyte/src/", glob="*.css")
```

Reference `references/ui-design-tokens.md` for token list.
</step>

<step n="5">
For new Pydantic models:

```
Grep(pattern="class .*\\(BaseModel\\)", path="src/", -n=true)
# Read context, verify ConfigDict(extra="forbid"), Field(description=...)
```
</step>

<step n="6">
Emit verdict using `assets/self-review-verdict-template.md`.
</step>

## Success Criteria
- [ ] All modified files identified via git diff
- [ ] Every taste rule has explicit Pass / Fail / Accepted / N/A
- [ ] Every Fail has file:line evidence
- [ ] Every Fail has minimal-patch recommendation
- [ ] Recommendation unambiguous (Apply / Apply N fixes / Block)
- [ ] No code modifications in review-only mode
- [ ] If fixes proposed, user confirms before transition to modification mode