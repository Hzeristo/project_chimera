# Taste Rules

The enforceable rules behind `core_principles` (SKILL.md). Each rule is a bare
statement followed by a Bad/Good pair. Anti-patterns (symptom → fix, no examples)
live in `anti_patterns.md`; UI token data lives in `ui_design_tokens.md`.

## layering_direction
**Statement:** `core` imports nothing internal. `ports` imports `core`. `services` imports both. Never reverse the arrow.

**Bad:**
```python
# core/schemas.py
from crucible.services.vault_service import VaultService  # core depends on services
```

**Good:**
```python
# services/vault_service.py
from crucible.core.schemas import ToolOutput   # services depends on core
from crucible.ports.vault_port import VaultPort
```

## function_naming
**Statement:** Name a function for what it does, not for the call path that reaches it. No `_handle_`/`_for_router_`/`_v2` path encoding.

**Bad:**
```python
def get_vault_note_for_router_tool_call(note_id: str) -> Note: ...
```

**Good:**
```python
def get_vault_note(note_id: str) -> Note: ...
```

## abstraction_threshold
**Statement:** Rule of three — an abstraction (helper, base class, generic) requires **3 concrete call sites**. Two is a coincidence; extract on the third.

**Bad:**
```python
# one caller, pre-emptive generalization
def _resolve_path(kind: str, *parts: str) -> Path: ...   # only ever called with kind="vault"
```

**Good:**
```python
# inline at the single call site until a third caller appears
note_path = vault_root / "notes" / f"{note_id}.md"
```

## exception_handling
**Statement:** No `except BaseException`. Catch specific types. Always re-raise `CancelledError` and `CLIENT_GONE_EXCEPTIONS` — swallowing them breaks async cancellation and SSE teardown.

**Bad:**
```python
try:
    await run_tool(call)
except BaseException:          # swallows CancelledError, hides bugs
    return ToolOutput.denied("error")
```

**Good:**
```python
try:
    await run_tool(call)
except asyncio.CancelledError:
    raise
except (ToolError, ValidationError) as exc:
    log.warning("[tool] denied: %s", exc)
    return ToolOutput.denied(str(exc))
```

## pydantic_defaults
**Statement:** A multi-field payload is a Pydantic model, not a `str` parsed internally. No `str` argument bag, no mutable default values.

**Bad:**
```python
def stage_node(payload: str) -> None:        # "kind|title|body" parsed inside
    kind, title, body = payload.split("|")
```

**Good:**
```python
class StageNodeArgs(BaseModel):
    kind: NodeKind
    title: str
    body: str = ""

def stage_node(args: StageNodeArgs) -> None: ...
```

## ui_tokens
**Statement:** UI uses only `--astrocyte-*` / `--surface-*` / `--space-*` / `--radius-*` tokens. No hex literals, no off-grid spacing, no emoji-as-icon. Token table and the full Forbidden list are in `ui_design_tokens.md` (the authority — do not duplicate values here).

**Bad:**
```css
.panel { background: #1a1a24; padding: 10px; border-radius: 5px; }
```

**Good:**
```css
.panel { background: var(--surface-1); padding: var(--space-3); border-radius: var(--radius-md); }
```

## logging_format
**Statement:** Every log line starts with a bracket-prefixed component tag, e.g. `[agent]`, `[vault]`, `[stage]`. No bare-string logs.

**Bad:**
```python
log.info("resumed task %s", task_id)
```

**Good:**
```python
log.info("[agent] resumed task %s", task_id)
```

## no_opportunistic_refactor
**Statement:** A sprint touches only its declared file list and mixes no refactor into feature work. "While I'm here" is a hard stop — log it, do not do it.

**Bad:**
```text
Sprint FC.2 (add staging telemetry) also renames StagingService.flush →
StagingService.commit across 6 files "while I'm here".
```

**Good:**
```text
Sprint FC.2 adds staging telemetry only. The flush→commit rename is noted as a
candidate for a separate refactor sprint and left untouched.
```
