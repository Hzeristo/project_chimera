# crucible_core/src/oligo/core/schemas.py
"""Oligo agent domain schemas — identity (Layer 2) + migrated Oligo domain models."""

from __future__ import annotations

import uuid
from datetime import datetime
from enum import Enum
from typing import Any, Literal

from pydantic import BaseModel, ConfigDict, Field, model_validator


# ─── Layer 2: Identity ───────────────────────────────────────────────────────

class TurnId(BaseModel):
    """Composite turn identifier: session + sequential turn number."""

    session_id: str
    turn_number: int

    model_config = ConfigDict(frozen=True, extra="forbid")

    @classmethod
    def create(cls, session_id: str | None, turn_number: int) -> TurnId:
        return cls(session_id=session_id or str(uuid.uuid4())[:8], turn_number=turn_number)

    def __str__(self) -> str:
        return f"{self.session_id}:{self.turn_number}"

    @classmethod
    def from_str(cls, s: str) -> TurnId:
        sid, tn = s.rsplit(":", 1)
        return cls(session_id=sid, turn_number=int(tn))

    def __hash__(self) -> int:
        return hash((self.session_id, self.turn_number))

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, TurnId):
            return NotImplemented
        return self.session_id == other.session_id and self.turn_number == other.turn_number


class TurnContext(BaseModel):
    """Per-turn metadata container. In-memory only, not persisted."""

    turn_id: TurnId
    turn_number: int
    started_at: datetime = Field(default_factory=datetime.now)

    model_config = ConfigDict(extra="forbid")


class ConversationContext(BaseModel):
    """Per-conversation metadata. Created once per AgentInvokeRequest."""

    session_id: str = Field(default_factory=lambda: str(uuid.uuid4()))
    active_persona_id: str | None = None
    active_skill_id: str | None = None

    model_config = ConfigDict(extra="forbid")


# ─── Migrated Oligo domain models (was crucible/core/schemas.py) ─────────────


class ToolCallStatus(str, Enum):
    """Discrete states for permission checks and execution outcomes of one tool call."""

    ALLOWED = "ALLOWED"
    DENIED = "DENIED"
    SUCCESS = "SUCCESS"
    ERROR = "ERROR"
    TIMEOUT = "TIMEOUT"


class ChatMessage(BaseModel):
    model_config = ConfigDict(extra="forbid")

    role: Literal["system", "user", "assistant", "tool"] = Field(
        ..., description="The author of this message."
    )
    content: str = Field(..., description="The textual content of the message.")
    tool_call_id: str | None = Field(
        default=None,
        description="Reserved for OpenAI Function Calling (tool result messages).",
    )
    name: str | None = Field(
        default=None,
        description="Reserved for OpenAI Function Calling (tool name).",
    )
    # A.1 Identity threading
    turn_id: str | None = Field(
        default=None,
        description="TurnId str for the turn that produced this message. None for system slot.",
    )


class PlannedToolCall(BaseModel):
    """Parsed tool invocation (XML / CMD) with optional allowlist gate and structured args."""

    model_config = ConfigDict(extra="forbid")

    id: str = Field(
        ...,
        description="Short unique id per invocation (e.g. UUID fragment) for logs and UI correlation.",
    )
    tool_name: str = Field(..., description="Registry tool name from the CMD tag.")
    raw_args: str = Field(
        ...,
        description="Literal text inside the CMD parentheses from the model output.",
    )
    args: dict[str, Any] = Field(
        ...,
        description="JSON object parsed from raw_args (same semantics as agent-side parsing).",
    )
    allowed: bool = Field(
        ...,
        description="True if policy allows execution for this tool name in the current context.",
    )
    deny_reason: str | None = Field(
        default=None,
        description="Human-readable reason when allowed is False; None when allowed.",
    )
    repairs_applied: list[str] = Field(
        default_factory=list,
        description="Conservative format-only repairs applied to raw_args before JSON parse (TP.3).",
    )


class Artifact(BaseModel):
    """A structured pointer to a side-channel resource produced by a tool run."""

    model_config = ConfigDict(extra="forbid")

    kind: str = Field(
        ...,
        description="Artifact category, e.g. 'vault_note', 'web_page'. UI selects icon by kind.",
    )
    path: str = Field(
        ...,
        description="Resource locator (vault-relative path or URL).",
    )
    metadata: dict[str, Any] | None = Field(
        default=None,
        description="Optional free-form metadata (title, snippet, score). Not used by the LLM.",
    )


class ToolOutput(BaseModel):
    """Opt-in structured tool return."""

    model_config = ConfigDict(extra="forbid")

    text: str = Field(
        ...,
        description="LLM-facing text. Wash policy applies as if this were a legacy str return.",
    )
    artifacts: list[Artifact] | None = Field(
        default=None,
        description="Optional side-channel pointers; consumed by FC.2 aggregation, never by the LLM.",
    )


class ExecutedToolResult(BaseModel):
    """Immutable record of one tool run: inputs, status, optional raw/wash text, timing."""

    model_config = ConfigDict(extra="forbid")

    call_id: str = Field(..., description="Matches PlannedToolCall.id for this execution.")
    tool_name: str = Field(..., description="Tool that was invoked.")
    args: dict[str, Any] = Field(
        ...,
        description="Structured arguments used for execution (copy of or canonical parse).",
    )
    status: ToolCallStatus = Field(..., description="Permission or runtime outcome.")
    raw_result: str | None = Field(default=None)
    washed_result: str | None = Field(default=None)
    error_message: str | None = Field(default=None)
    elapsed_ms: int | None = Field(default=None)
    artifacts: list[Artifact] | None = Field(default=None)
    # A.4 long-running task identity
    task_id: str | None = Field(
        default=None,
        description="Set when a long_running tool spawned a background task.",
    )
    # A.1 Identity threading
    turn_id: str | None = Field(
        default=None,
        description="TurnId str for the turn that produced this result.",
    )


class OligoAgentConfig(BaseModel):
    """Tunable tool execution deadlines and wash routing policy for ChimeraAgent."""

    model_config = ConfigDict(extra="forbid")

    tool_execution_deadline_seconds: float = Field(
        default=45.0, ge=1.0, le=600.0,
    )
    wash_min_chars: int = Field(default=1200, ge=0)
    bypass_wash_tools: set[str] = Field(
        default_factory=lambda: {
            "search_vault_attribute",
            "metadata_lookup",
            "planner_json",
        },
    )
    force_wash_tools: set[str] = Field(
        default_factory=lambda: {
            "search_vault",
            "web_search",
            "read_markdown",
        },
    )


class PromptStage(str, Enum):
    """Prompt 注入的目标阶段"""

    ROUTER = "router"
    FINAL = "final"
    BOTH = "both"
    MESSAGE_INJECTION = "message_injection"


PromptRenderer = Literal["text", "xml_structured"]


class PromptComponent(BaseModel):
    """单个 Prompt 片段的元数据"""

    model_config = ConfigDict(extra="forbid")

    id: str = Field(description="唯一标识, 如 'router_core', 'skill_directive'")
    stage: PromptStage
    priority: int = Field(description="数字越大越靠前. 100=核心规则, 50=任务约束, 10=guardrail")
    cacheable: bool = Field(default=True)
    renderer: PromptRenderer = Field(default="text")
    template: str | dict[str, Any] = Field(
        description="text 模式为含占位符的字符串；xml_structured 模式为可嵌套的 dict。",
    )

    @model_validator(mode="after")
    def _renderer_matches_template_kind(self) -> PromptComponent:
        if self.renderer == "xml_structured":
            if not isinstance(self.template, dict):
                raise ValueError("renderer 'xml_structured' requires template to be a dict[str, Any]")
        elif not isinstance(self.template, str):
            raise ValueError("renderer 'text' requires template to be a str")
        return self


class AgentInvokeRequest(BaseModel):
    """Agent 调用请求体（与 Astrocyte ``OligoAgentRequest`` JSON 严格同构）。"""

    model_config = ConfigDict(extra="forbid")

    api_key: str = Field(...)
    base_url: str = Field(...)
    model_name: str = Field(...)
    temperature: float | None = Field(default=None, ge=0.0, le=2.0)
    persona_id: str | None = Field(default=None)
    system_core: str = Field(...)
    skill_override: str | None = Field(default=None)
    skill_id: str | None = Field(default=None)
    allowed_tools: list[str] | None = Field(default=None)
    messages: list[ChatMessage] = Field(...)
    persona: str | None = Field(default=None)
    authors_note: str | None = Field(default=None)
    # A.1: session identity (optional — Python side generates UUID if absent)
    session_id: str | None = Field(
        default=None,
        description="Caller-supplied session identity; auto-generated if absent.",
    )

class AgentPhase(str, Enum):
    """Thin label for the current async step. Observation only — does NOT
    drive control flow (the coroutine flow is the machine)."""
    ROUTING = "routing"            # probe call + parse CMD/PASS (incl. system-slot setup)
    EXECUTING = "executing"        # tool dispatch + gather
    AWAITING_TASK = "awaiting_task"  # suspended on long-running task completion (A.4)
    WASHING = "washing"            # tool result compression
    RENDERING = "rendering"        # compose messages for next turn
    SYNTHESIZING = "synthesizing"  # Final: persona bind + generate + stream (incl. probe backfill)

class RouteResult(BaseModel):
    """ROUTING step output. Narrow — only what downstream steps need."""
    probe_response: str
    planned_calls: list[PlannedToolCall] = Field(default_factory=list)
    wash_context: str | None = None      # captured at routing, consumed by washing
    probe_for_cmd: str | None = None     # DSL-stripped probe, for history backfill
    is_trivial: bool = False             # no-tool branch: trivial vs draft-worthy

    model_config = ConfigDict(extra="forbid")


class ExecuteResult(BaseModel):
    """EXECUTING step output."""
    executed_results: list[ExecutedToolResult]
    has_long_running: bool = False       # signals A.4 whether to enter AWAITING_TASK

    model_config = ConfigDict(extra="forbid")

class TerminalReason(str, Enum):
    """How a turn coroutine terminates. Returned by the turn, never a Phase."""
    COMPLETED = "completed"            # SYNTHESIZING finished normally
    TURN_EXHAUSTED = "turn_exhausted"  # max_turns reached without resolution
    CLIENT_GONE = "client_gone"        # client disconnected mid-turn
    LLM_TIMEOUT = "llm_timeout"        # LLM gateway exceeded deadline
    TASK_FAILED = "task_failed"        # long-running task failed (A.4)


class TurnOutcome(str, Enum):
    """A turn either continues the loop (tools ran) or terminates it."""
    CONTINUE = "continue"   # tools executed, loop to next turn
    TERMINATE = "terminate" # synthesized final answer OR hit a terminal reason