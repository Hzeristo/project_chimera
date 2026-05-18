# Project Chimera

Personal research OS for a single PhD-student user. Not a framework. Not SaaS.

## Start here
- Current phase state: `docs/ROADMAP.md`
- Active frictions: `docs/FRICTION_LOG.md`
- Accepted trade-offs: `docs/ACCEPTED_PARTIALS.md`
- Open debt: `docs/TECHNICAL_DEBT.md`

## Skills
1. `chimera-core-philosophy` — always active
2. `chimera-sprint-discipline` — planning / reviewing
3. `chimera-code-taste` — writing / editing code
4. `chimera-dependency-veto` — adding dependencies
5. `chimera-commit-style` — drafting commits

## Hard rules
- This repo has ONE user. Do not generalize.
- Never modify files in the "Start here" list without explicit in-conversation user approval.
- Skill rules override generic best practices.

## Architecture
See `docs/ARCHITECTURE/`.

## Development environment

### Python (crucible_core)
- Path: `<TBD — populate after debt-week cleanup>`
- Version: `<TBD>`
- Package manager: `<TBD>`
- Lockfile: `<TBD>`

**Until populated**: Claude must ask user before running pytest/ruff/mypy.
After populated: Claude prefixes all tool calls with the declared path.

### Rust (astrocyte)
- Standard cargo workspace, no special config
- Lockfile: `astrocyte/Cargo.lock` is authoritative

