# Skill Self-Assessment — Signal-to-Noise & Internal Discipline

**Date:** 2026-06-27
**Scope:** All 5 Chimera skills (SKILL.md + every `references/*.md`) plus the
assets and `scripts/check_taste.ps1` they bind to.
**Mode:** Catalog only. No fixes applied. The user decides what to act on.
**Action vocabulary:** Keep (noise but harmless) · Move (to ARCHITECTURE/ or
phase doc) · Tighten (rewrite as pure instruction + verification) · Internalize
(delegated check → direct check) · Consolidate (merge duplicates).

## Files audited

| Skill | SKILL.md | reference docs | lines (refs) |
|---|---|---|---|
| chimera-core-philosophy | 67 L | — | — |
| chimera-commit-style | 122 L | — | — |
| chimera-dependency-veto | 67 L | — | — |
| chimera-sprint-discipline | 180 L | phase-audit (53), batch-planning (60), phase-review (192), rules_and_antipatterns (67) | 372 |
| chimera-code-taste | 164 L | batch_execution (116), taste_rules (32), anti_patterns (22), ui_design_tokens (43) | 213 |

Two skills (core-philosophy, dependency-veto) are **pure declarative reference**
with no process docs — they are not "execution" skills, so dimensions B/C/D/G
largely do not apply to them. They are evaluated under A/E/F only.

---

## ⚠️ Two integrity defects found before scoring dimensions

These are not "low signal" — they are **broken signal**, and they distort every
density metric below. Listed first because they have the highest leverage.

| ID | Location | Finding | Action |
|---|---|---|---|
| **X1** | `chimera-code-taste/references/taste_rules.md` (whole file) | The canonical taste-rules reference is a **half-finished migration stub**. Lines 7-32 are literal placeholders: `[from chimera-code-taste 旧 SKILL.md 的 layering_direction bad block]`, `[同上 good block]`, `[同样的迁移]`, and bare `...`. Only line 4 carries a real statement. The actual rules now live scattered across `SKILL.md` core_principles (79-86) + `anti_patterns.md`. SKILL.md `rules_summary` (line 146) **points users to this empty file** as "full rules with bad/good examples." A subagent told to "apply taste_rules.md" gets nothing. | **Tighten** — either finish the migration (move the real DDD-layering / naming / exception / pydantic-default rule bodies in with bad/good blocks) or delete the file and repoint line 146. Do not leave a referenced-but-empty rules doc. |
| **X2** | `chimera-code-taste/SKILL.md:11`; `references/batch_execution_process.md:44`; `scripts/check_taste.ps1:3` | **Path mismatch on the commit gate.** The script lives at `.claude/skills/chimera-code-taste/scripts/check_taste.ps1`, but the allowed-tools glob says `./scripts/check_taste.ps1:*` and the script's own usage comment says `./scripts/check_taste.ps1`. There is no repo-root `scripts/` dir. A main-session call to `./scripts/check_taste.ps1` would fail path resolution — which silently *forces* the Haiku delegation (B-dimension) rather than it being a free choice. The script also hardcodes `tests/oligo/test_$base.py` (line 37), valid only from a specific cwd. | **Tighten** — fix the allowed-tools glob to the real path and make the script cwd-explicit; this unblocks the internalization option in B1. |

---

## A. Signal density (instruction_lines / total_lines; flag < 30%)

Instruction = a step that tells Claude what to *do* or *verify*. Tables,
rationale, examples, background are not instructions. Counts are line estimates.

| Doc | ~instr / total | density | Verdict |
|---|---|---|---|
| batch_execution_process.md | ~78 / 116 | ~67% | OK |
| phase-audit-process.md | ~36 / 53 | ~68% | OK |
| batch-planning-process.md | ~42 / 60 | ~70% | OK |
| phase-review-process.md | ~110 / 192 | ~57% | OK overall (but see A2) |
| anti_patterns.md | ~20 / 22 | ~91% | OK (model density) |
| **taste_rules.md** | **~1 / 32** | **~3%** | **FLAG → X1** |
| **ui_design_tokens.md** | **~8 / 43** | **~19%** | **FLAG → A1** |
| **rules_and_antipatterns.md** | **~14 / 67** | **~21%** | **FLAG → A2** |

| ID | Location | Finding | Action |
|---|---|---|---|
| **A1** | `ui_design_tokens.md` (whole) | 19% by the metric, but the doc is a **pure lookup table** (color/spacing/radius tokens) + a Forbidden list (40-43). A token table *should* be low-"step" — the metric mis-fires on reference data. The Forbidden list (the only real "verify" content) is the part code-taste actually enforces. | **Keep.** Noise by the metric, correct by purpose. Optionally relabel the file `references/ui_token_reference.md` to signal "data, not process." Lesson: dimension-A thresholds should exempt pure-data tables. |
| **A2** | `rules_and_antipatterns.md` (Bad/Good blocks, ~40 of 67 L) | Each rule is 1 statement line + a Bad block + a Good block. The examples are ~⅔ of the file. For a *rules* reference the examples are pedagogically load-bearing (they disambiguate "≤3 files / ≤50 lines" etc.), so this isn't pure noise — but it drags density to 21%. | **Keep** the examples (they earn their place), but **Tighten** by leading each rule with the bare imperative so a skimming subagent gets the rule before the illustration. |

**Cross-cutting A note:** the `<key_insight>` preamble on all four process docs
(batch_execution 3-7, phase-audit 3-7, batch-planning 3-8, phase-review 3-10) is
100% rationale by design. Acceptable as framing, but it is the single largest
recurring non-instruction block — see F1.

---

## B. Verification internalization (flag every delegated check)

All three verification *executions* are delegated to Haiku subagents; all three
*decisions* are internalized. This is a deliberate, consistent pattern (context
economy + cost), recently hardened for code-taste (commits bd7b375, 4e1bb52).

| ID | Location | What is delegated | Decision internalized? | Could it be internalized? | Action |
|---|---|---|---|---|---|
| **B1** | code-taste `batch_execution_process.md:43-49` + SKILL.md 60-77 | Running `check_taste.ps1` (ruff+mypy+pytest) via Haiku subagent | **Yes** — main session reads the **exit code** (0/≠0), explicitly told prose is non-authoritative. This is the gold-standard delegation in the repo. | **Yes, mechanically.** The script is deterministic and exit-code-driven; it's in allowed-tools (once X2 is fixed). Internalizing removes one hop and the risk a subagent fabricates/omits the exit code (no halt path exists if it does — see C5). The *only* reason to delegate is keeping verbose ruff/mypy output out of main context. | **Internalize (conditional).** For small sprints (≤3 files) the output is short — run directly in main session. Keep delegation only as a context-budget escape valve, and say so explicitly. Net: this is the *best* current pattern; the question is whether the hop still earns its keep now that the decision is already internalized. |
| **B2** | sprint-discipline `phase-review-process.md:60-70` | Repo-wide red-line Grep, one subagent doing ALL patterns, returns "file:line of any matches" | **Partially.** Verdict/categorization is internalized (step 4), but the returned file:line list is **trusted as-is** — no re-verification, no exit-code/empty-vs-error distinction. Unlike B1, there is no "subagent output is not authoritative" contract here. | **Yes.** Red-line scans are literal `Grep` calls (Grep is in allowed-tools). For a phase-seal — the highest-stakes gate — running the red-line Grep **directly** is cheap and removes the trust gap. | **Internalize.** Red-line verification at *seal time* is too important to delegate without the B1-style "re-verify, don't trust prose" contract. Either internalize the Grep, or extend the B1 contract (return raw match lines; main session re-Greps the flagged files). |
| **B3** | sprint-discipline `phase-audit-process.md:26` ("Spawn subagent for repo-wide pattern scans") + SKILL.md 86-98 | Audit pattern scans across many files | Verdict (risk rating, file:line answer) internalized in step 4 | Partially — bulk file scanning across a whole phase genuinely benefits from fan-out; this is the one place delegation clearly pays. | **Keep**, but tighten the return contract (see G). Audit scans are read-only and high-volume; delegation is justified here more than at seal time. |

**B summary:** The decision-internalization is sound everywhere. The asymmetry
to fix: code-taste has a hardened "exit code is authoritative, prose is not"
contract (B1); sprint-discipline's two delegations (B2, B3) **trust returned
prose/file:line without re-verification**. Port the B1 contract to B2/B3, and
internalize B2 outright since it's the seal gate.

---

## C. Halt completeness (processes with a commit/write step)

Only `batch_execution` commits. `phase-review` writes+stages (explicitly **zero
commits**, step 8 / success criteria). audit & planning are read-only.

### batch_execution_process.md — enumerated halts

Covered: red-line violation (step 5b → halt entire batch); check_taste exit ≠ 0
(step 5a → no commit, surface); broken predecessor assumptions on resume (step 0
→ halt); context boundary at ~70% (step N → handoff).

| ID | Missing failure mode | Where it should halt | Action |
|---|---|---|---|
| **C1** | **Preconditions have no STOP verb.** Lines 9-13 list "batch plan exists / user invoked / env declared" but never say *what happens if false*. Compare phase-review:19 ("If any precondition fails, STOP. Output diagnosis."). | Before step 0 | **Tighten** — add the explicit STOP sentence batch_execution is missing. Asymmetry with the other three process docs that all have it. |
| **C2** | **Dirty / conflicted working tree before editing.** No `git status` clean-check before step 3 edits. Per-sprint commit isolation (the whole trust model, key_insight) breaks if uncommitted junk rides along. | New precondition / step 0 | **Tighten** — add "working tree clean (or only batch-owned files staged) before first edit; else HALT." Highest-leverage C gap: it protects the commit-isolation invariant the skill is built on. |
| **C3** | **Out-of-date base branch.** No check that the branch is current before a batch of commits. | Precondition | **Keep/Tighten (low pri)** — single-user repo, low collision risk; a one-line "note base branch" is enough. The user's own dimension-C prompt named this; flagged for completeness, but lowest leverage here. |
| **C4** | **Commit step can fail.** Step 6 ("Commit with Tier-2 message") assumes success. No halt if `git commit` returns non-zero (hook failure, empty diff, etc.). | step 6 | **Tighten** — "if commit fails, HALT, do not proceed to next sprint." A failed commit mid-batch silently desyncs the summary record (step 6 writes the summary *then* commits). |
| **C5** | **Subagent returns no/garbage exit code.** The contract (SKILL.md 71-77) says "decide from exit code" but defines no behavior if the exit code is absent/unparseable. Default could be "assume pass." | step 4/5 | **Tighten** — "missing or non-integer exit code is treated as FAIL (halt), never as pass." Closes the one hole in the otherwise-excellent B1 contract. |
| **C6** | **Edit failure / file-not-in-scope.** Step 3 assumes edits apply. No halt if a target file moved or an Edit fails. | step 3 | **Keep (low pri)** — Edit tool errors are already surfaced by the harness; a note suffices. |

### phase-review-process.md — halts

Well covered: explicit precondition STOP (19); "Fail" category → block sealing
(step 4); ❌ NOT Sealed decision (step 7). One gap:

| ID | Missing | Action |
|---|---|---|
| **C7** | **Edit/Out-File conflict during auto-apply (step 8).** Appends to ACCEPTED_PARTIALS/TECHNICAL_DEBT assume the write succeeds; step 8 says "verify by Read after write" (good) but no halt instruction if the post-write Read does not show the entry. | **Tighten** — "if post-write Read does not confirm the entry, HALT and surface; do not stage." The verify exists; the halt-on-verify-fail does not. |

---

## D. Approval gates vs. automation (declared authority vs. process behavior)

| ID | Location | Finding | Action |
|---|---|---|---|
| **D1** | `phase-review-verdict-template.md:70-98` vs `phase-review-process.md:117-140` + SKILL.md 63-84 | **Template contradicts declared authority.** The verdict template puts **ACCEPTED_PARTIALS.md, TECHNICAL_DEBT.md, and friction-log updates** under a section titled *"Proposed State File Updates — Apply these diffs manually after reviewing"* (line 72). But the process and SKILL.md both class those three as **AUTO-APPLY (no user approval)**. The template tells the user to hand-apply what the process auto-applies. A model following the template will gate writes the authority says to make automatically (or vice-versa). | **Tighten / Consolidate.** Split the template's state section into "Auto-applied (already staged): ACCEPTED_PARTIALS, TECHNICAL_DEBT, friction-flips" vs "Proposed for approval: ROADMAP, reclassifications." Single highest-leverage D fix — it's an active contradiction in the seal path. |
| **D2** | `phase-review-process.md:96-106` (step 6) vs step 8 auto-apply scope | **OPEN→RESOLVED is unauthorized.** Step 6 checks whether friction status moved from **OPEN/SCHEDULED** to RESOLVED. But auto-apply (step 8 + SKILL.md 70) only authorizes **SCHEDULED→RESOLVED**. A friction that was OPEN (never scheduled) and got resolved this phase falls in a gap: step 6 expects it resolved, step 8 has no auto authority to flip it, and propose-diff covers "changes that aren't direct phase-resolutions" — which this *is*. Ambiguous. | **Tighten** — either widen auto-apply to "OPEN/SCHEDULED → RESOLVED for frictions this phase resolved," or have step 6 only contemplate SCHEDULED. Pick one; today they disagree. |
| **D3** | code-taste SKILL.md 76-80 ("arch docs: chimera-code-taste only, in sprint scope") + Write in allowed-tools | **Arch-doc writes ride the auto-commit.** code-taste may Write architecture docs within sprint scope, and batch_execution auto-commits everything in scope (step 6) with no separate gate. Defensible (batch plan is pre-approved), but ARCHITECTURE/ changes are higher-stakes than code and currently get the same ungated auto-commit. | **Keep**, but **note** — consider surfacing "this sprint modified an ARCHITECTURE/ doc" in the per-sprint summary so the seal review (which traces audit→impl) can see doc drift. Not a contradiction, a visibility gap. |
| **D4** | code-taste batch_execution model | **Per-sprint auto-commit with no per-sprint approval** is correct and matches the declared model (key_insight: "trust comes from commit isolation, not per-sprint approval"). No mismatch. | **Keep** — documented for completeness; this is by design. |

---

## E. Redundancy across skills (blocks > 5 lines in multiple skills)

| ID | Block | Locations | Size | Action |
|---|---|---|---|---|
| **E1** | `<execution_environment>` | sprint SKILL.md 108-140 **≡** code-taste SKILL.md 88-120 | ~32 L verbatim | **Consolidate.** Already flagged (F12) with manual SYNC comments. Two 32-line verbatim copies kept in sync by hand is the single largest redundancy. Extract to a shared include or a `references/_execution_environment.md` both skills point to. |
| **E2** | `<incident_protocol>` | sprint SKILL.md 142-163 **≡** code-taste SKILL.md 122-143 | ~22 L verbatim | **Consolidate.** Same SYNC-comment situation as E1. |
| **E3** | `<subagent_routing>` policy | sprint SKILL.md 86-98 ≈ code-taste SKILL.md 60-77 | ~10 L shared shape | **Consolidate (partial).** Both open "Spawn subagents (Agent tool, general-purpose, model: Haiku) for: …" + "Do NOT spawn for: …" + a return policy. code-taste's return *contract* is richer (exit code) — keep that part skill-specific, but the shared "what to/not-to delegate + model:Haiku + return structured not verbatim" preamble is duplicated. |
| **E4** | `<expected_model>` mechanism | sprint SKILL.md 31-51 ≈ code-taste SKILL.md 36-58 | ~20 L each, shared logic | **Consolidate (mechanism only).** The *mechanism* is identical: model table → "Output before any other work" recommendation → "Wait for confirmation" → "Do NOT auto-switch." Only the table contents differ. Extract the boilerplate procedure; keep the per-skill table. |
| **E5** | Audit-path naming convention (`docs/audits/{prerequisite-sprint-id}.md`, general `{phase}.0.md`) | phase-audit 39-46, batch-planning 11, phase-review 14, verdict-template 3 | ~3-7 L restated 4× | **Consolidate.** The same naming rule + examples is re-explained in four places. Put it once (e.g. a "Path Conventions" block in sprint-discipline SKILL.md or a tiny shared reference) and link. Drift risk is real — the explanation already varies slightly between copies. |
| **E6** | Case-asymmetry note (lowercase `phase-{X.Y}.md` docs vs capital-P `Phase-{X.Y}-batch.md` plans) | batch-planning 45-51, referenced phase-review 15 | ~7 L | **Consolidate** into the same Path Conventions block as E5. |
| **E7** | Hard-precondition + STOP idiom | bootstrap (sprint 20-29), phase-audit 9-13, batch-planning 10-14, phase-review 12-19, batch_execution 9-13 | short each | **Keep** but standardize wording. Not worth extracting (short, context-specific), but note C1: batch_execution is the one copy missing the STOP verb — standardizing fixes that for free. |

**E leverage:** E1+E2 alone are ~54 lines of hand-synced duplication across two
skills — the highest-volume consolidation. E5+E6 are the highest *drift-risk*
(path strings that must match disk exactly, restated 4×).

---

## F. "Why" vs "What" separation (flag steps where explanation > instruction)

| ID | Location | Finding | Action |
|---|---|---|---|
| **F1** | `<key_insight>` on all 4 process docs (batch_execution 3-7, phase-audit 3-7, batch-planning 3-8, phase-review 3-10) | 100% rationale. Useful framing, but it's the largest recurring "why" block and sets a pattern of explanation-in-process-docs. | **Keep** (preamble is fine) — but cap at ~3 lines. phase-review's is 8 lines; trim. |
| **F2** | `batch_execution_process.md:43-49` (step 4) | The return-contract explanation (~5 L of "do not trust prose, exit code is authoritative") **exceeds** the actual instruction ("spawn subagent, run check_taste, read exit code"). Worse, it **duplicates** the contract already stated in SKILL.md 71-77. | **Move/Tighten.** The contract belongs in SKILL.md (where it already is). Step 4 should be a one-liner pointing to it: "Run check_taste via Haiku per the subagent return contract (SKILL.md); read exit code." |
| **F3** | `batch_execution_process.md:51-66` (step 5) | Halt conditions interleaved with rationale ("a sprint that does not verify cannot be sealed", "Green is proven by exit code 0, not asserted"). | **Keep (mostly).** This rationale is anti-bypass and load-bearing — it's *why the model must not skip the halt*. Borderline; trim the most repetitive line but don't strip it. |
| **F4** | `phase-audit-process.md:38-46` (step 6) | Naming-convention explanation (40-46, ~7 L) dwarfs the instruction ("output using template, write to path"). | **Move.** Send the naming rule to the shared Path Conventions block (E5); leave step 6 as the bare write instruction. |
| **F5** | `batch-planning-process.md:45-51` (step 5) | Case-asymmetry explanation (~6 L) dwarfs the instruction ("write to `docs/plans/{phase}-batch.md`"). | **Move** to Path Conventions (E6). |
| **F6** | `phase-review-process.md:148-180` (Examples, bad/good) | 33 lines of illustration — 0% instruction. Valuable as a model of a good verdict, but it lives inside the *process* doc and largely **restates the verdict template**. | **Move** to the verdict template (or an adjacent `examples/` note). The process doc should say "see template for worked example." |
| **F7** | `chimera-core-philosophy/SKILL.md` (whole) | ~95% "why." | **Keep.** This is a declarative philosophy skill, not a process — explanation *is* its payload. Flagged only to record that the F-criterion is N/A here by design. |

---

## G. Subagent fan-out / parallelization quality

How parallelizable is each process, and how good are the skills at (i) explicit
fork specs, (ii) compact structured returns, (iii) avoiding redundant subagents?

| Process | Parallelizable? | Current quality | Action |
|---|---|---|---|
| **phase-audit** (`phase-audit-process.md:16-46`) | **High.** Step 3 scans across a whole phase's in-scope files; each sprint's 1-3 audit questions (step 2) drives an independent repo scan. Natural fan-out: **one Haiku per audit question** (or per file group), main session synthesizes step 4. | **Poor at fork spec.** Step 3 is one vague line: "Spawn subagent for repo-wide pattern scans." No fan-out, no per-Q forking, no input/output schema beyond "file:line." No dedup guidance (overlapping scans across questions re-read the same files). | **Tighten + parallelize.** Spec a fan-out: N questions → N Haiku scouts, each given `{question, file globs, patterns}`, each returns a fixed schema `{question_id, hits:[{file,line,snippet}], risk}`. Dedup file reads by grouping questions that share globs. Highest G leverage — audit is the most fan-out-friendly process and currently the least specified. |
| **phase-review** (`phase-review-process.md:34-106`) | **Medium-high.** Step 3 verifies acceptance per *completed sprint* (independent) + a repo-wide red-line scan. Both fan out: one verifier per sprint, one scanner per red-line pattern. Currently sequential, single subagent for the scan, main-session loop for acceptance. | **Poor.** Single inline `Agent()` example (62-69) bundles ALL red-line patterns into one prompt returning loose prose. No per-sprint parallelism, no schema, no "subagent output non-authoritative" contract (see B2). | **Tighten + parallelize** — but pair with B2: at seal time, prefer **internalized** Grep over fan-out for red lines (correctness > speed at the gate). Acceptance-per-sprint verification *can* fan out to Haiku with a `{sprint_id, criterion, pass, evidence}` schema. |
| **batch-planning** (`batch-planning-process.md:17-52`) | **Low by policy.** Step 2 derives objective/scope/red-lines/risk per sprint — independent in principle. But sprint SKILL.md 92-95 **explicitly forbids** subagenting planning decisions ("reasoning IS the work"). | N/A — deliberately serial. A *drafting* fan-out is possible (Haiku drafts file-scope scaffolding from the audit, Opus decides), but currently foreclosed. | **Keep** the no-delegate-decisions rule. **Note** the unused option: Haiku could pre-populate audit-derived file lists per sprint (mechanical) for Opus to ratify — a draft/decide split, not a decision delegation. Low priority. |
| **batch_execution** (`batch_execution_process.md:14-93`) | **Low by design.** Sprints commit serially with predecessor dependencies (commit isolation = trust model). Cannot parallelize across sprints. Within a sprint, ruff+mypy+pytest are already bundled into one check_taste call. | Fine. The one delegated step (B1) has the best contract in the repo. | **Keep.** Correctly serial. Minor: step 0 resume verification ("Git log + targeted Greps") could fan out per predecessor assumption, but the win is marginal. |

**G summary — skill-wide weaknesses in subagent usage:**
1. **No structured return schemas anywhere.** Returns are described as prose
   ("file:line of violations", "structured summaries"). Only code-taste's exit-code
   contract is enforced. → Define JSON-ish return schemas per delegation type.
2. **No explicit fan-out.** Every delegation is a single sequential spawn even
   where the work is embarrassingly parallel (audit scans, per-sprint review).
   → Where read-only and independent, fan out and `.filter`/synthesize.
3. **No redundant-subagent / dedup guidance.** Overlapping scans re-read the same
   files. → Group by shared file globs before forking.
4. **Inconsistent "non-authoritative" contract.** Only B1 states the subagent's
   prose is not the decision. B2/B3 trust returned prose. → Port the contract.

---

## Prioritized opportunity list (by leverage, not ease)

Leverage = affects multiple skills, unblocks automation, or protects an
invariant the skill is built on.

| Rank | ID(s) | Why it's top leverage |
|---|---|---|
| 1 | **X1** | A *referenced* rules doc is empty. The code-taste skill claims to enforce rules whose canonical text is placeholder. Fixing this restores the skill's core payload. |
| 2 | **X2** | The commit-gate script path is wrong, silently forcing the B1 delegation and risking a no-op gate. Fixing unblocks B1 internalization and makes the gate runnable from main session. |
| 3 | **D1** | Active contradiction in the seal path: the verdict template tells the user to hand-apply what the process auto-applies. Touches the highest-stakes (phase-seal) flow. |
| 4 | **E1 + E2** | ~54 lines of hand-synced verbatim duplication across two skills. One extraction kills the SYNC-comment maintenance burden permanently. |
| 5 | **C2 (+C1, C4, C5)** | batch_execution's halt set has holes around its own trust invariant (commit isolation): no clean-tree check, no precondition STOP verb, no commit-failure halt, no missing-exit-code halt. |
| 6 | **B2** | The phase-seal red-line check trusts subagent prose with no contract. Internalize it — seal-time correctness > speed. |
| 7 | **G (audit fan-out)** | The audit process is the most parallel-friendly and least specified. A fork-spec + schema unlocks real parallel speedup with no correctness cost (read-only). |
| 8 | **E5 + E6** | Path/case conventions restated 4× and already drifting — highest silent-breakage risk among the redundancies. |
| 9 | **F2, F4, F5, F6** | Move "why"/naming/examples out of process steps into SKILL.md, Path Conventions block, or templates — raises density without losing content. |
| 10 | **D2** | OPEN→RESOLVED authority gap; narrow but a real ambiguity in auto-write authority. |

**Lowest leverage (record, don't rush):** A1 (metric mis-fire, Keep), C3/C6
(single-user, low collision), D3/D4 (by-design, visibility note only), F1/F3
(trim, don't strip), G batch-planning (deliberately serial).

---

*Catalog only — no fixes applied. Generated by an open-domain self-audit of the
5 Chimera skills against dimensions A-G.*
