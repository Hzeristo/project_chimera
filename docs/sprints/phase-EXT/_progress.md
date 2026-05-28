# Phase EXT — Sprint Progress

| Sprint | Status | Commit | Notes |
|---|---|---|---|
| EXT.0 | Sealed | — | Audit only; `docs/audits/EXT.md` |
| EXT.1 | Sealed | `10a282a` | Prompt externalization; 9 template files; byte-lock 2492→2594 |
| EXT.2a | Pending | — | Remove 4000-char cap + budget-shrink loop; update cap test |
| EXT.2b | Pending | — | Verify user-provided router_intro.md.j2 renders; update byte-lock |
| EXT.2c | Pending | — | Add router_continuation.md.j2 + theater loop turn-based swap |
| EXT.2d | Pending | — | Strip `<thinking>` tags from probe_response before tool call parsing |
| EXT.3 | Pending | — | Rich ToolSpec: user_aliases + common_mistakes fields + verbose render |
| EXT.4 | Pending | — | Architecture discussion only; no code |

## Blocking notes

- **EXT.2b content delivered** (2026-05-28): User provided `router_intro.md.j2` and updated `final_guardrail.md.j2` via direct file edit.
- EXT.2a → EXT.2b → EXT.2c → EXT.2d sequential.
- EXT.3 can run after EXT.1 (no dependency on EXT.2).
