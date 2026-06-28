# Skill Set Re-Audit (post-fix)

**Date:** 2026-06-27
**Predecessor:** `docs/audits/skill-self-assessment.md` (the A–G + X1/X2 catalog)
**Scope:** Re-verify that the applied fixes hold, re-score the original
dimensions on current disk state, and add a common-sense skill-requirements lens
(skill anatomy: triggering, progressive disclosure, frontmatter, self-containment).
**Method note:** These five are subjective workflow/discipline skills, not
file-transform tools — there is no objectively gradable output to run an
eval-prompt/benchmark loop against, so this is a structured re-audit, not a
skill-creator eval cycle. The one skill-creator capability that *does* apply
here — description-triggering optimization — is offered at the end.

---

## Part 1 — Verification of applied fixes

Every prior finding that was actioned, re-checked against current disk state.

| ID | Fix | Status | Evidence |
|---|---|---|---|
| X1 | taste_rules.md migration | ✅ Resolved | 8 real rule sections w/ Bad/Good; sweep for `同上/旧 SKILL/...` stubs → 0 hits. `rules_summary` (code-taste SKILL.md:103) points at real files. |
| X2 | check_taste path + cwd | ✅ Resolved | Glob → `.claude/skills/chimera-code-taste/scripts/check_taste.ps1` (SKILL.md:11); script resolves repo root from `$PSScriptRoot`, runs from `crucible_core/` in try/finally; `ParseFile` OK. |
| C1 | precondition STOP verb | ✅ Resolved | batch_execution_process.md: "If any precondition fails, STOP." |
| C4 | commit-fail HALT | ✅ Resolved | step 6 HALTs on commit failure. |
| C5 | missing exit code = FAIL | ✅ Resolved | code-taste SKILL.md:75 + step-5(a). |
| C7 | post-write Read-fail HALT | ✅ Resolved | phase-review-process.md step 8.2. |
| B2 | internalize seal red-line Grep | ✅ Resolved | phase-review step 3 runs Grep directly; Haiku only pre-collects, main re-Greps. |
| G | audit fan-out schema | ✅ Resolved | phase-audit step 3 one-scout-per-question + return schema + dedup. |
| D1 | verdict-template state split | ✅ Resolved | template: Auto-applied vs Proposed-for-approval. |
| D2 | OPEN/SCHEDULED authority | ✅ Resolved | SKILL.md:59 + phase-review step 8 both say OPEN/SCHEDULED→RESOLVED. |
| E1/E2 | execution-env + incident extract | ✅ Resolved | `_shared/*.md` exist; 0 `SYNC:` comments remain. |
| E3/E4 | routing + model boilerplate | ✅ Resolved | shared policy + per-skill table/contract kept. |
| E5/E6 | path conventions | ✅ Resolved | `path_conventions.md`; 3 process docs cite it. |
| F2/F4/F5/F6 | move why/naming/examples | ✅ Resolved | step 4 cites SKILL.md contract; naming → conventions; examples → template appendix. |

**Pointer integrity sweep:** all 16 distinct `../_shared/`, `references/`, and
`assets/` targets referenced by the two process skills resolve to real files.
No dangling pointers. SKILL.md sizes 66–127 lines (budget is ~500) — healthy.

---

## Part 2 — New findings (introduced by the fixes, or surfaced by the deeper lens)

| ID | Sev | Location | Finding | Action |
|---|---|---|---|---|
| **N1** | **HIGH** | `chimera-sprint-discipline/SKILL.md:31-41` | **Regression from the E4 extraction.** The `<expected_model>` block lost its inline activation trigger. It now reads only "Recommendation procedure … see ../_shared/expected_model.md" + the table. The "on activation, if the model is wasteful, output the recommendation **before any other work** and wait" imperative lives ONLY in the shared file — which the model has no reason to open at activation. code-taste KEPT an inline trigger (SKILL.md:48-50: "If the current model is Opus, before any other work output the cost note … and wait"); sprint-discipline did not. Net effect: sprint-discipline's model-recommendation may silently never fire. | **Tighten (fix)** — restore a one-line inline trigger naming the firing condition, e.g. "On activation, if the current model is Wasteful for the selected mode, follow the recommendation procedure in ../_shared/expected_model.md before any other work." This is a regression I introduced; recommend fixing now. |
| **N2** | MED | both SKILL.md (`../_shared/*` refs) | **Self-containment / portability.** The shared pointers reach OUTSIDE the skill directory. Fine in-repo (Read tool, same repo, allowed-tools has Read), but: (a) `scripts/package_skill.py` bundles only the skill dir, so an *exported* skill would carry dangling `../_shared/` pointers; (b) progressive-disclosure caveat — pointer content loads only if the model reads it, which is correct for on-demand reference (idiom list, full incident protocol) but is exactly the trap behind N1 for must-fire-on-activation content. | **Keep + document.** These skills are repo-local, not distributed, so the dedup win outweighs portability. Add one line to each `_shared/*.md` header (already present) and accept non-export. The real action is N1 (don't put activation-time triggers behind a pointer). |
| **N3** | LOW | `chimera-code-taste` description + invocation_modes (SKILL.md:3, 28-34) | **Trigger scope vs CLAUDE.md framing.** code-taste triggers only on batch/sprint phrasing ("execute sprint/batch", "run FC.N..M"). CLAUDE.md's skill list frames it as "writing / editing code" generally. An ad-hoc request ("fix the naming in vault_tools.py") pulls in no taste enforcement. Likely intentional (skill is batch-scoped; incidents bypass), but it's a genuine under-trigger gap given the LLM under-triggering tendency. | **Decide** — either broaden the description to cover single-file taste passes, or accept batch-only and soften CLAUDE.md's "writing/editing code" line to "batch sprint execution". Recommend the latter (keeps the skill focused). CLAUDE.md edit needs user approval (it's a Hard-rule protected file). |
| **N4** | LOW | `chimera-code-taste/SKILL.md:48` | The inline expected_model trigger checks only "If the current model is Opus." The table also marks Haiku as insufficient for reading/editing, but no inline trigger warns when the active model is Haiku. Unlikely in practice. | **Keep** (note) — or generalize the trigger to "if current model is Wasteful for the operation" to match N1's wording and the table. |
| **N5** | LOW | `taste_rules.md` vs core_principles (code-taste SKILL.md:79-86) | X1 reintroduced controlled redundancy: rule *names* appear in both (1-liners in core_principles, full Bad/Good in taste_rules). This is intended summary-vs-detail layering, NOT the E-class duplication that was removed. | **Keep** — flagged only so it isn't "consolidated" by mistake. The layering is correct progressive disclosure. |
| **N6** | LOW | `chimera-core-philosophy`, `chimera-dependency-veto` (no allowed-tools); `chimera-commit-style` (Edit) | The two advisory skills declare no `allowed-tools` → default broad access; they are read-only advisory, so least-privilege (`Read/Grep/Glob`) would be tidier but is harmless. commit-style grants `Edit` though drafting a commit message edits no tracked file — possible minor over-grant. | **Keep / optional tighten** — least-privilege hygiene, not a correctness issue. |

---

## Part 3 — Common-sense skill-requirements scorecard

Skill-anatomy checks (skill-creator: progressive disclosure, triggering, frontmatter).

| Requirement | Verdict | Notes |
|---|---|---|
| `name` + `description` present (all 5) | ✅ | — |
| SKILL.md < ~500 lines | ✅ | 66 / 66 / 120 / 121 / 127 |
| Description states *what* + *when* | ✅ | All have when-to-use clauses. |
| Sibling skills disambiguate | ✅ | sprint-discipline says "Code modifications are delegated to chimera-code-taste"; code-taste says "during sprint execution within a batch". |
| Progressive disclosure (SKILL → references → assets) | ✅ | Clean hierarchy; pointers are explicit. Exception: N1 puts must-fire content behind a pointer. |
| Reference pointers resolve | ✅ | 16/16 targets exist. |
| bootstrap targets exist | ✅ | code-taste's 4 `docs/architecture/*.md` all present (lowercase dir matches). |
| No placeholder/stub content | ✅ | Post-X1 sweep clean. |
| Triggering "pushiness" (anti-under-trigger) | ⚠️ | Descriptions are accurate but not pushy. The process skills trigger on explicit mode phrases; the advisory skills ("Activate when …") are reasonable. N3 is the one real under-trigger gap. Candidate for description-optimization (Part 4). |
| Self-contained / packageable | ⚠️ | N2 — `../_shared/` breaks export. Acceptable for repo-local use. |

---

## Part 4 — Applicable skill-creator tooling: description optimization

The eval-prompt/benchmark loop doesn't fit subjective workflow skills, but
**description-triggering optimization** (`skill-creator/scripts/run_loop.py`)
does — it measures and improves how reliably each skill fires on realistic
queries (train/held-out split, 3× per query). It's the right tool for the N3
under-trigger gap and the general "not pushy" observation.

Best candidates, in priority order:
1. **chimera-code-taste** — N3 trigger-scope gap; verify batch-only is the
   intended boundary and that near-miss ad-hoc edits behave as designed.
2. **chimera-dependency-veto** — high value to fire on indirect cues ("let's
   just add langchain for the agent loop") without the user naming a dependency.
3. **chimera-sprint-discipline** — confirm the three mode phrases (incl. the
   Chinese ones 前置审计 / 终审) trigger reliably and don't cross-fire with code-taste.

This needs the `claude -p` CLI and a reviewed 20-query eval set per skill. Say
the word and I'll draft the eval sets for your sign-off, then run the loop in the
background.

---

## Bottom line

All 14 prior fixes verified resolved; pointer integrity and progressive-
disclosure budgets are healthy. **One regression (N1) was introduced by the
shared-extraction commit** and should be fixed: activation-time model
recommendation is now behind a pointer in sprint-discipline and may not fire.
Everything else is low-severity hygiene or an accepted trade-off (N2 portability,
N5 intended layering). N3 is the only behavioural design question worth a
decision.

*Re-audit only — no fixes applied in this pass (N1 fix offered, awaiting go-ahead).*
