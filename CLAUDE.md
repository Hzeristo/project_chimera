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
- Path: `crucible_core/.venv/Scripts/python.exe`
- Version: 3.11+
- Package manager: uv
- Manifest: `crucible_core/pyproject.toml`
- Lockfiles: `crucible_core/requirements.txt`, `crucible_core/requirements-dev.txt`
- Activation prefix for tool calls: `Bash("./crucible_core/.venv/Scripts/python -m {tool}")`

### Rust (astrocyte)
- Standard cargo workspace, no special config
- Lockfile: `astrocyte/Cargo.lock` is authoritative

