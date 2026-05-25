# FC.6 — E2E Smoke Procedure

**Phase:** III.C | **Sprint:** FC.6 (seal sprint)  
**Date authored:** 2026-05-25  
**Purpose:** Manual end-to-end verification of E3 (vault chip → Obsidian) and E4 (message delete persistence) before phase seal.

No automated E2E framework introduced — `chimera-dependency-veto` applies. This checklist is the smoke record; run once on the dev machine and mark each item.

---

## Prerequisites

- Astrocyte running (`cargo tauri dev` or release build)
- `~/.chimera/config.toml` has `vault_root` pointing to a real Obsidian vault
- At least one vault note exists that a vault tool can return (e.g. a note matching a known keyword)
- Oligo backend running and connected

---

## Smoke A — Vault Tool → Artifact Chip → Obsidian Open (E3)

| # | Step | Expected | Result |
|---|------|----------|--------|
| A1 | Send a message that triggers a vault tool (e.g. "search vault for X" where X matches a known note) | BB response arrives; tool executes | |
| A2 | Inspect the BB message in the chat | One or more `◈ filename` chips appear below the message text | |
| A3 | Click a chip | Obsidian opens to the corresponding note | |
| A4 | Click a chip whose path has been manually tampered (via devtools: set `art.path` to `../../../etc/passwd`) | Error appears in system log (`[OPEN_VAULT_NOTE_ERROR]`); no crash; Obsidian does not open | |
| A5 | Kill and restart Astrocyte; reopen the same session | The BB message still shows the artifact chips (persistence round-trip) | |

---

## Smoke B — Message Delete Persistence (E4)

| # | Step | Expected | Result |
|---|------|----------|--------|
| B1 | In an existing session, click the Delete (D) button on a BB message | Message disappears from the chat immediately | |
| B2 | Kill and restart Astrocyte; reopen the same session | The deleted message is gone; surrounding messages intact | |
| B3 | Attempt to delete a stage card (if any visible) | Delete button is absent on stage cards (only `user`/`bb` rows show msg-actions) | |

---

## Smoke C — HSC Grep Verification

Run these from the repo root. All must pass before seal.

```powershell
# HSC #1 — vault tools return ToolOutput
grep -c "ToolOutput" crucible_core/src/oligo/tools/vault_tools.py
# Expected: ≥ 3

# HSC #2 — artifacts never in LLM payload
grep "artifacts" crucible_core/src/oligo/core/agent.py | grep -E "messages\.append|_render_tool_results_for_llm|_format_one_tool_result_xml"
# Expected: no output

# HSC #3 — Router persona-invariant
cd crucible_core && .venv/Scripts/python.exe -m pytest tests/oligo/test_prompt_composer.py -k persona -q
# Expected: 2 passed
```

---

## Sign-off

Once all items above are marked, hand off to `chimera-sprint-discipline` `phase_review` mode for seal verdict.
