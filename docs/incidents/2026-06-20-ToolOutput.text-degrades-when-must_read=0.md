# 2026-06-20 — V.A.2b ToolOutput.text degrades when must_read=0

**Status:** Open (hotfix scheduled)
**Phase:** V.A (functionally complete, pending HSC-2 verification)

## Symptom

daily_paper_pipeline live run with all 3 papers rejected → BB final
reply lists aggregate counts only ("must_read=0 skim=0 reject=3"), no
paper titles. HSC-2 ("BB lists real titles, not terse summary") fails.

## Root cause (suspected, pending audit)

V.A.2b builds Artifact list only for must_read items
(_collect_pipeline_artifacts iterates stats.must_read_items). When
must_read=0, artifacts=[]. Behavior under empty artifacts is the open
question:
  (a) ToolOutput is still constructed with text=summary, artifacts=[],
      but text is the OLD terse counts string (V.A.2a enrichment never
      applied because all items were rejected) — design oversight
  (b) ToolOutput is NOT constructed at all when artifacts=[] (early
      return to plain str) — wiring bug

Either way, downstream Router sees a plain-counts string, not a
title-listing enriched summary.

## Design decision (user, 2026-06-20)

ToolOutput.text MUST always be enriched — list all filtered papers'
titles regardless of verdict (must_read / skim / reject). Artifacts
remain selective (only papers with a vault path: must_read + skim,
not reject — reject papers go to papers/filtered/Reject/, not vault).

This decouples "what BB says" from "what frontend can chip-link":
  - text: enriched summary listing ALL 3 papers (titles + verdicts)
  - artifacts: 0-2 chips (only vault-resident papers)

## Acceptance

- BB reply after pipeline lists every processed paper's title +
  verdict, even when must_read=0
- Artifacts list contains exactly the must_read+skim papers with
  resolvable vault paths
- arxiv_miner path (legacy plain-string result) unchanged

## Files likely touched

- crucible_core/src/crucible/services/daily_chimera_service.py
  (_collect_pipeline_artifacts and surrounding ToolOutput construction)
- crucible_core/src/oligo/tools/miner_tools.py
  (verify check_task_status fallback unchanged)
