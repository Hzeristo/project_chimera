# FC.6 — Documentation + E2E smoke (seal sprint)

- **Commit:** (this record commit)
- **Status:** Pass
- **Files changed:** `docs/ARCHITECTURE/FINAL_CONTRACT.md` (new), `docs/audits/FC.6-e2e-smoke.md` (new)

## What was done

**`docs/ARCHITECTURE/FINAL_CONTRACT.md`** (~130 lines)
Documents all contracts established by FC.1–FC.5:
1. `ToolOutput` / `Artifact` shape and the invariant that artifacts never enter LLM payload
2. `bb-message-artifacts` SSE event — emit site, payload shape, ordering invariant
3. Tauri forward + persistence boundary (`user + bb` only; `state::Message` artifact-free)
4. Persona stage rule — `final_persona_override` is FINAL-only; Router secondary guard; test coverage
5. Path-containment rule for `open_vault_note` — two-layer guard (pre-filesystem + post-canonicalize)
6. Chip rendering contract — tokens-only CSS, no inline preview, error routing
7. Message delete pipeline boundary — full component table + stage card semantics

**`docs/audits/FC.6-e2e-smoke.md`**
Manual smoke procedure (no new E2E framework — `chimera-dependency-veto`):
- Smoke A: vault tool → chip render → Obsidian open → tamper test → persistence round-trip
- Smoke B: message delete → restart → confirm gone → stage card non-deletability
- Smoke C: HSC grep verification commands with expected outputs

## HSC verification (at commit time)

- HSC #1: `grep -c "ToolOutput" vault_tools.py` → 18 hits ✓
- HSC #2: `grep "artifacts" agent.py | grep "messages.append\|_render_tool_results_for_llm"` → no output ✓
- HSC #3: `pytest -k persona` → 2/2 PASS ✓

## Accepted partials

- E2E manual smoke (Smokes A + B) deferred to user execution — no automated E2E harness exists and none introduced per `chimera-dependency-veto`
- ROADMAP.md, FRICTION_LOG.md, ACCEPTED_PARTIALS.md diffs proposed separately (propose-diff category per `<state_write_authority>`)

## Planning deviation

None. `FINAL_CONTRACT.md` follows style of `TOOL_PROTOCOL.md` / `PROMPT_MIDDLEWARE.md` as specified.
