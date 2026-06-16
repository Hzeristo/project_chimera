# V.A.6 E2E Smoke Checklist

**Phase:** V.A  
**Date:** (fill in at execution time)  
**Executor:** (user)  
**Precedent:** `docs/audits/FC.6-e2e-smoke.md`

E2E smoke is a manual checklist, not an automated harness (ACCEPTED_PARTIALS FC.6.1).

---

## Pre-conditions

- [ ] Astrocyte running (dev or production build)
- [ ] Oligo backend running (`start_oligo.py`)
- [ ] `vault_root` configured in `~/.chimera/config.toml`
- [ ] Vault `templates/` populated: copied `Tpl_{knowledge,thought,insight,decision}.md` from `crucible_core/prompts/obsidian_tpl/` (V.A.1 user action)
- [ ] At least one paper in `papers/` that will be triaged as `must_read`

---

## Smoke #1 — Templates (Sealing Condition 1)

- [ ] Open Obsidian → Templates folder → confirm `Tpl_knowledge.md`, `Tpl_thought.md`, `Tpl_insight.md`, `Tpl_decision.md` exist
- [ ] Open `Tpl_thought.md` → confirm `graph_edges` block has `derives_from`, `supersedes`, `contradicts`, `dead_ends` (spec names, no legacy names)
- [ ] Open `Tpl_decision.md` → confirm `depends_on` and `dead_ends` edges present

**Result:** PASS / FAIL / PARTIAL  
**Notes:**

---

## Smoke #2 — Structured Output (Sealing Condition 2)

- [ ] Send to BB: `daily_paper_pipeline(skip_telegram=True)`
- [ ] BB returns `[Task Started]` with task_id
- [ ] Send: `check_task_status("<task_id>")`
- [ ] BB reply contains actual paper titles (e.g. `FooNet [8/10]`), NOT just `must_read=1 skim=2`
- [ ] If Astrocyte artifact chips visible: chip shows vault note filename for each must_read paper

**Result:** PASS / FAIL / PARTIAL  
**Notes:**

---

## Smoke #3 — Staging Flow (Sealing Condition 3)

- [ ] BB generates a response containing a hypothesis or observation
- [ ] Hover the BB message → "N" button appears in action row
- [ ] Click "N" → confirm `docs/staging/*.md` created with `status: PENDING_REVIEW`
- [ ] Open Astrocyte staging panel → candidate appears in list
- [ ] Click Promote → file disappears from staging panel → appears in `vault_root/Thoughts/` with `status: active`
- [ ] Repeat with a second candidate → click Reject → file removed from staging, not in vault

**Result:** PASS / FAIL / PARTIAL  
**Notes:**

---

## Smoke #4 — vault_query (Sealing Condition 4)

- [ ] Send to BB: `vault_query(type="knowledge")`
- [ ] Response arrives in < 2s (approximate, by observation)
- [ ] Response contains at least one result with title + path + type/status excerpt
- [ ] Send: `vault_query(type="thought", status="dead_end")` → only dead_end thoughts in results (0 results is acceptable if no dead_end thoughts exist yet)

**Result:** PASS / FAIL / PARTIAL  
**Notes:**

---

## Summary

| Smoke | Condition | Result |
|---|---|---|
| #1 | Templates deployed to vault | |
| #2 | Structured output — real titles in BB reply | |
| #3 | Staging flow — create / promote / reject | |
| #4 | vault_query — latency + filter correctness | |

**Overall seal verdict:** PASS / CONDITIONAL PASS / BLOCKED  
**Blocking issues (if any):**
