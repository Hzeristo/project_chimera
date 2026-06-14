# IV.A.1 — Identity DDD (Layer 2)

- **Commit:** TBD (pending commit)
- **Status:** Sealed
- **Files changed:**
  - `crucible_core/src/oligo/core/schemas.py` — **new file**: `TurnId`, `TurnContext`, `ConversationContext` + all migrated Oligo domain models (`ChatMessage`, `PlannedToolCall`, `ExecutedToolResult`, `OligoAgentConfig`, `ToolCallStatus`, `Artifact`, `ToolOutput`, `PromptStage`, `PromptRenderer`, `PromptComponent`, `AgentInvokeRequest`)
  - `crucible_core/src/crucible/core/schemas.py` — Oligo model bodies removed; re-exports added; `TaskEvent.turn_id` added
  - `crucible_core/src/oligo/core/agent.py` — `session_id` param; `ConversationContext` constructed at `__init__`; `TurnContext` + `TurnId.create` per loop iteration; `turn_id=str(_turn_ctx.turn_id)` threaded into 3 `ChatMessage` append sites
  - `crucible_core/src/oligo/api/server.py` — `AgentInvokeRequest` import migrated; `session_id=body.session_id` passed to agent
  - `crucible_core/src/crucible/core/config.py` — `OligoAgentConfig` import migrated
  - `crucible_core/src/oligo/core/prompt_composer.py` — `PromptComponent`, `PromptStage` imports migrated
  - `crucible_core/src/oligo/core/text_sanitizer.py` — `ChatMessage` import migrated
  - `crucible_core/src/oligo/tools/registry.py` — `PlannedToolCall`, `ToolOutput` imports migrated
  - `crucible_core/src/oligo/tools/vault_tools.py` — `Artifact`, `ToolOutput` imports migrated

## What was done

**New identity layer** (`oligo/core/schemas.py`):
- `TurnId` — frozen composite (`session_id: str`, `turn_number: int`); `create(session_id, turn_number)` factory; `__str__` → `"{sid}:{n}"`; `from_str` round-trip; `__hash__`/`__eq__` for dict keys
- `TurnContext` — per-turn container (`turn_id: TurnId`, `turn_number: int`, `started_at: datetime`); placeholder for A.2 state fields
- `ConversationContext` — per-session container (`session_id: str`, `active_persona_id`, `active_skill_id`); placeholder for A.2 state history

**Field additions (default=None, backward-compatible):**
- `ChatMessage.turn_id: str | None = None`
- `ExecutedToolResult.turn_id: str | None = None`
- `TaskEvent.turn_id: str | None = None`
- `AgentInvokeRequest.session_id: str | None = None`

**Agent wiring** (`agent.py`):
- `__init__` accepts `session_id: str | None`; builds `self._conversation_ctx = ConversationContext(session_id=session_id)` (or auto-UUID if None)
- Loop top: `_turn_ctx = TurnContext(turn_id=TurnId.create(self._conversation_ctx.session_id, turn), turn_number=turn)`
- Three `ChatMessage(...)` append sites: `agent.py:1334`, `1346`, `1357` — all pass `turn_id=str(_turn_ctx.turn_id)`

**ExecutedToolResult construction sites not yet threaded** — expected; A.1 is schema-only. Full threading through the executor is A.3 scope.

## HSC verification

- `turn_id` fields on `ChatMessage`, `ExecutedToolResult`, `TaskEvent`: grep confirms all present with `default=None`
- `ConversationContext` at `agent.py:430`; `TurnContext` at `agent.py:1165`; 3 threading sites at `1334`, `1346`, `1357`
- 0 remaining `from src.crucible.core.schemas import` for Oligo models in consumers (grep clean)
- Framework red line: 0 imports of LangGraph/Temporal/Celery
- Wire format: all new fields are `str | None = None` — old clients unaffected
- `pytest tests/oligo/`: 5 pre-existing failures, 0 regressions
