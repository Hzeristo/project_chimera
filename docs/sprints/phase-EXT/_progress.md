# Phase EXT — Sprint Progress

| Sprint | Status | Commit | Notes |
|---|---|---|---|
| EXT.0 | Sealed | — | Audit only; `docs/audits/EXT.md` |
| EXT.1 | Sealed | `10a282a` | Prompt externalization; 9 template files; byte-lock 2492→2594 |
| EXT.2a | Pending | — | Structural scaffolding: remove cap, placeholder router_intro.md.j2 headers, update byte-lock |
| EXT.2b | Awaiting content | — | User to deliver pre-written router_intro.md.j2 content |
| EXT.3 | Pending | — | Rich ToolSpec: user_aliases + common_mistakes fields + verbose render |
| EXT.4 | Pending | — | Architecture discussion only; no code |

## Blocking notes

- **EXT.2b content delivered** (2026-05-28): User has provided `router_intro.md.j2` content via file edit. `final_guardrail.md.j2` also updated by user. EXT.2a can proceed.
- EXT.2a must precede EXT.2b paste-in.
- EXT.3 can run after EXT.1 (no dependency on EXT.2).
