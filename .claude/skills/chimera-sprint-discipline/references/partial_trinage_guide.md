# Partial Triage Decision Guide

When review reveals a Partial, run this decision tree:

## Step 1: Was the trade-off declared upfront?

**Yes** → was it declared in the sprint prompt's 红线 or 设计要点 section?
- Yes, declared → **Accepted Partial**. Append to ACCEPTED_PARTIALS.md.
- No, agent's interpretation → continue to Step 2.

**No** → continue to Step 2.

## Step 2: Does the Partial violate a red line?

**Yes** → **Fail**. Block merge. Propose minimal patch (<30 lines).

**No** → continue to Step 3.

## Step 3: Does the Partial break existing behavior?

**Yes** → **Fail**. Block merge.

**No** → continue to Step 4.

## Step 4: Is the trade-off acceptable in principle?

Ask: "If a future maintainer reads this in 6 months, will they understand why we accepted it?"

**Yes** → **Accepted Partial**. Document reason in 1 line.

**No** → **Technical Debt**. File DEBT-{id} with priority and resolution plan.

## Examples

| Partial | Decision | Reason |
|---|---|---|
| Tool list compresses to micro mode under length budget | Accepted | Length budget harder than args visibility |
| Zero-arg emission not asserted vs live LLM | Accepted | Requires live-model CI infrastructure |
| `except BaseException` introduced | Fail | Red line violation |
| Async tests missing pytest.mark.asyncio | Technical Debt | Resolvable in debt week, low priority |
| Tool fade animation 0.35s instead of declared 1s | Accepted | UI animation budget; functional behavior unchanged |

