"""
剧场版 ReAct 引擎：静默推理层 + 终极推流层。

废除工具思考阶段的流式截断，采用「前期阻塞探包 + 后期全量推流」架构。
内部消息流严格使用 list[ChatMessage]，在边界处通过 model_dump 与网络层对接。
"""

from __future__ import annotations

import asyncio
import errno
import json
import logging
import re
import time
from datetime import datetime
from typing import Any, AsyncGenerator, Literal

from collections.abc import Awaitable, Callable

from src.oligo.core.schemas import (
    AgentPhase,
    Artifact,
    ChatMessage,
    ConversationContext,
    ExecuteResult,
    ExecutedToolResult,
    OligoAgentConfig,
    PlannedToolCall,
    PromptStage,
    RouteResult,
    ToolCallStatus,
    ToolOutput,
    TurnContext,
    TurnId,
)
from src.crucible.ports.llm.base import LLMClient
from src.crucible.services.metrics_service import MetricsService
from src.oligo.core.prompt_composer import _load_md, _render_tool_list, get_prompt_composer
from src.oligo.core.sse import sse_event
from src.oligo.core.text_sanitizer import TextSanitizer
from src.oligo.core.tool_protocol import (
    extract_tool_syntax_for_history,
    parse_args_with_repair,
    parse_tool_calls_unified,
    planned_call_id_from_parsed,
)
from src.oligo.tools import TOOL_REGISTRY
from src.oligo.tools.registry import get_tool_registry, partition_tool_calls
from src.crucible.services.task_service import get_task_service
from src.crucible.core.schemas import TaskEventType
logger = logging.getLogger(__name__)

CLIENT_SEVERED_WARNING = (
    "[Oligo] Client forcibly severed the connection. "
    "Neural loop aborted mid-flight. Purging memory."
)


def _client_gone_exception_types() -> tuple[type[BaseException], ...]:
    """Exceptions that mean the HTTP/SSE client dropped — swallow quietly (no traceback spam)."""
    types_list: list[type[BaseException]] = [
        asyncio.CancelledError,
        ConnectionError,
        BrokenPipeError,
        ConnectionResetError,
        ConnectionAbortedError,
    ]
    try:
        from starlette.requests import ClientDisconnect  # type: ignore[attr-defined]

        types_list.append(ClientDisconnect)
    except ImportError:
        pass
    return tuple(types_list)


CLIENT_GONE_EXCEPTIONS: tuple[type[BaseException], ...] = _client_gone_exception_types()


def _looks_like_pipe_broken(exc: BaseException) -> bool:
    """Uvicorn/ASGI sometimes surfaces broken pipes as OSError or RuntimeError."""
    if isinstance(exc, OSError):
        if exc.errno in (errno.EPIPE, errno.ECONNRESET, errno.ECONNABORTED):
            return True
    msg = str(exc).lower()
    if "disconnect" in msg or "connection reset" in msg or "broken pipe" in msg:
        return True
    if "client" in msg and "disconnect" in msg:
        return True
    return False


def _handle_client_gone() -> None:
    logger.warning(CLIENT_SEVERED_WARNING)


# 工具调用字面量：``tool_protocol``（XML + CMD）；剥离代码块见 ``TextSanitizer``（S0.4）


def _strip_markdown_code_for_cmd_extraction(text: str) -> str:
    """
    从 router probe 中去掉 Markdown 代码块/行内代码，再用于工具调用匹配。

    避免模型在 `` ``` `` 或 `` `...` `` 里**举例** ``<CMD:...>`` 时被误执行。

    委托 :meth:`TextSanitizer.strip_code_blocks_for_tool_matching`。
    """
    return TextSanitizer.strip_code_blocks_for_tool_matching(text)


def _strip_router_dsl_for_backfill(text: str) -> str:
    """
    在 draft backfill 写入 ``self.messages`` 前去掉可见区路由 DSL 字面量（代码块内保留举例）。

    委托 :meth:`TextSanitizer.strip_tool_syntax_in_visible`。
    """
    return TextSanitizer.strip_tool_syntax_in_visible(text)


_THEATER_LLM_OUTER_TIMEOUT_S = 120.0
# Intent-Driven Wash: fallback when LLM call fails (chars + suffix length budget)
_WASH_FALLBACK_CHARS = 1500
_WASH_TRUNC_SUFFIX = "...[TRUNCATED]"
# Router-visible dialogue tail: last two user/assistant rounds (max 4 messages)
_WASH_CONTEXT_MAX_MESSAGES = 4
_WASH_CONTEXT_PER_MSG_CAP = 8000

# Router 探针：极短/仅 ``<PASS>`` 则不回填 assistant（避免把噪声当作草稿塞给 Final）
_ROUTER_TRIVIAL_MAX_CHARS = 30

# --- IR.2: LLM-facing tool result XML + reflection hints (render-only taxonomy) ---
_TR_REASON_DENIED = "DENIED"
_TR_REASON_TIMEOUT = "TIMEOUT"
_TR_REASON_TOOL_ERROR = "TOOL_ERROR"
_TR_REASON_ARGS_INVALID = "ARGS_INVALID"
_TR_REASON_EMPTY_RESULT = "EMPTY_RESULT"

_REFLECTION_HINT_FAILURE = (
    "Some tools failed. Consider: (a) retry with different args, (b) use an alternative tool, "
    "(c) tell the user honestly."
)
_REFLECTION_HINT_EMPTY = (
    "Empty result. Consider broadening query or trying web_search as fallback."
)

_USER_EXPECTATION_KEYWORDS: tuple[str, ...] = (
    "find",
    "search",
    "look up",
    "lookup",
    "where",
    "which",
    "whether",
    "any note",
    "locate",
    "有没有",
    "查找",
    "搜索",
    "找",
)

_EMPTY_SUCCESS_MARKERS: tuple[str, ...] = (
    "[no content]",
    "no results found",
    "no nodes found",
    "[graph query] no nodes",
    "[web_search] no results",
    "no matching notes",
    "no relevant information found",
    "[wash result]: no relevant information found",
)


def _xml_attr_escape(value: str) -> str:
    return (
        (value or "")
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace('"', "&quot;")
    )


def _user_text_suggests_expectation(user_text: str) -> bool:
    s = (user_text or "").strip().lower()
    if len(s) < 3:
        return False
    return any(k.lower() in s for k in _USER_EXPECTATION_KEYWORDS)


def _message_suggests_args_invalid(text: str) -> bool:
    m = (text or "").lower()
    needles = (
        "invalid args",
        "requires a non-empty",
        "must be non-empty",
        "invalid path",
        "typeerror",
        "failed to output json object",
    )
    return any(n in m for n in needles)


def _success_payload_behaves_like_tool_failure(body: str) -> bool:
    """Tools may return error prose with HTTP 200-style SUCCESS rows; surface as failure in render."""
    s = (body or "").strip()
    if not s:
        return False
    if s.lower().startswith("error:"):
        return True
    return s.startswith("[Tool Error]") or s.startswith("[Tool Timeout]")


def _body_looks_empty_for_hint(body: str) -> bool:
    s = (body or "").strip().lower()
    if not s or s == "[no content]":
        return True
    return any(marker in s for marker in _EMPTY_SUCCESS_MARKERS)


def _classify_render_outcome(
    er: ExecutedToolResult,
    result_body: str,
) -> tuple[Literal["success", "failed"], str | None]:
    """
    Map execution row + body text to LLM-facing wrapper status and ``reason`` attribute.

    Does not mutate ``ExecutedToolResult``; render-only.
    """
    st = er.status
    if st == ToolCallStatus.DENIED:
        return "failed", _TR_REASON_DENIED
    if st == ToolCallStatus.TIMEOUT:
        return "failed", _TR_REASON_TIMEOUT
    if st == ToolCallStatus.ERROR:
        blob = " ".join(
            x
            for x in (
                result_body,
                er.error_message or "",
                er.raw_result or "",
            )
            if x
        )
        if _message_suggests_args_invalid(blob):
            return "failed", _TR_REASON_ARGS_INVALID
        return "failed", _TR_REASON_TOOL_ERROR

    if st == ToolCallStatus.SUCCESS:
        if _success_payload_behaves_like_tool_failure(result_body):
            if _message_suggests_args_invalid(result_body):
                return "failed", _TR_REASON_ARGS_INVALID
            return "failed", _TR_REASON_TOOL_ERROR
        if _body_looks_empty_for_hint(result_body):
            return "failed", _TR_REASON_EMPTY_RESULT
        return "success", None

    return "failed", _TR_REASON_TOOL_ERROR


def _format_one_tool_result_xml(
    er: ExecutedToolResult,
    display_status: Literal["success", "failed"],
    reason: str | None,
    result_body: str,
) -> str:
    inner_lines = [
        f"--- Tool Call {er.call_id} ---",
        f"Tool: {er.tool_name}",
        f"Record status: {er.status.name}",
        f"Args: {json.dumps(er.args, ensure_ascii=False)}",
        "Result:",
        result_body,
    ]
    inner = "\n".join(inner_lines)
    cid = _xml_attr_escape(er.call_id)
    if display_status == "success":
        open_tag = f'<tool_result status="success" call_id="{cid}">'
    else:
        r = reason or _TR_REASON_TOOL_ERROR
        open_tag = (
            f'<tool_result status="failed" reason="{_xml_attr_escape(r)}" '
            f'call_id="{cid}">'
        )
    return f"{open_tag}\n{inner}\n</tool_result>"


def _is_router_pass_or_trivial(probe_response: str) -> bool:
    """为 True 时不把 probe 写入 messages（走纯 Final 合成）。"""
    s = (probe_response or "").strip()
    if not s:
        return True
    if s.upper() == "<PASS>":
        return True
    if len(s) < _ROUTER_TRIVIAL_MAX_CHARS:
        return True
    return False

# 工具环专用：动态路由 System 由 ``ChimeraAgent._build_router_system_prompt`` 生成，
# 与 ``TOOL_REGISTRY`` 及 ``allowed_tools`` 对齐，与晚期绑定的 BB / skill 人设分离。

def _sys_telemetry_obj(obj: dict[str, Any]) -> str:
    """Astrocyte 分段反馈：`__SYS_TOOL_CALL__` + JSON（stage / content / tool_name / decision）。"""
    return "__SYS_TOOL_CALL__" + json.dumps(obj, ensure_ascii=False)


def _tool_batch_start_payload(batch: list[PlannedToolCall]) -> dict[str, Any]:
    """TP.4：工具批调度遥测（并发批 / 串行批）。"""
    return {
        "stage": "tool",
        "phase": "batch_start",
        "batch_size": len(batch),
        "concurrency": "parallel" if len(batch) > 1 else "serial",
        "tools": [c.tool_name for c in batch],
    }


def _sse_data(payload: str) -> str:
    """
    将负载打包为单条 SSE data 帧；正文经 JSON 转义，换行与特殊字符不会破坏帧边界。

    Args:
        payload: 原始文本负载，可为空。

    Returns:
        ``data: {"content": "..."}\\n\\n`` 形式；空负载时为 ``data: \\n\\n``。
    """
    if not payload:
        return "data: \n\n"
    safe_json = json.dumps({"content": payload})
    return f"data: {safe_json}\n\n"


# 与 ``_sse_data`` 区分：本函数发出 **命名事件** ``bb-stream-chunk``，供下游（如 Rust）与 ``bb-stream-done`` 分轨解析。
def _sse_chunk(payload: str) -> str:
    return sse_event("bb-stream-chunk", {"content": payload})


def _messages_to_api(messages: list[ChatMessage]) -> list[dict[str, Any]]:
    """
    将 Pydantic 消息列表安全转化为网络层（OpenAI 等）接受的字典列表。

    仅保留非 None 字段，满足 API 契约。

    Args:
        messages: 强类型 ChatMessage 列表。

    Returns:
        适用于 chat.completions.create 的 messages 参数格式。
    """
    return [m.model_dump(exclude_none=True) for m in messages]


def _ensure_chat_messages(messages: list[dict[str, Any]] | list[ChatMessage]) -> list[ChatMessage]:
    """
    将外部输入的 list[dict] 或 list[ChatMessage] 强转为 list[ChatMessage]。

    用于 __init__ 的防御式校验，确保内部状态绝对类型安全。

    Args:
        messages: 来自 API 或上层的消息列表，可能是裸 dict。

    Returns:
        经 model_validate 校验后的 list[ChatMessage]。
    """
    if not messages:
        return []
    first = messages[0]
    if isinstance(first, ChatMessage):
        return list(messages)
    return [ChatMessage.model_validate(m) for m in messages]


class ChimeraAgent:
    """
    剧场版 ReAct Agent：路由环（短 System）+ 晚期人设绑定 + 全量推流。

    工作原理简述：
    1. 工具搜寻环：使用动态路由 System（与 ``TOOL_REGISTRY`` / ``allowed_tools`` 一致；
       可选追加 ``[Skill Directives]``）+ 纯净
       ``raw_messages``；非流式 ``generate_raw_text`` 探包，避免 ``<CMD>`` 截断。
    2. 晚期绑定：一旦本轮回答不含 ``<CMD>``，丢弃路由 System，以 L1（``system_core`` +
       可选 ``[SKILL DIRECTIVE]``）及 L2/L3 分层拼成最终 System 置于历史顶端（含已注入的
       ``[SYSTEM TOOL RESULTS]``），再发起**最后一次** ``generate_raw_text``，再推流。
    3. 边界处通过 ``model_dump`` 与网络层对接；异常与 SSE 帧格式保持不变。
    """

    def __init__(
        self,
        raw_messages: list[dict[str, Any]] | list[ChatMessage],
        system_core: str,
        skill_override: str | None,
        llm_client: LLMClient,
        wash_client: LLMClient | None = None,
        router_client: LLMClient | None = None,
        max_turns: int = 5,
        allowed_tools: list[str] | None = None,
        agent_config: OligoAgentConfig | None = None,
        persona: str | None = None,
        authors_note: str | None = None,
        metrics_service: MetricsService | None = None,
        session_id: str | None = None,
    ) -> None:
        """
        Args:
            raw_messages: 纯净 user/assistant 对话（无网关预拼 System）。
            system_core: L1 人设基座；仅在最终入模前与 L2/L3 分层拼接（不含 Author's Note 预混）。
            skill_override: 技能覆写文案；参与路由环 ``[Skill Directives]`` 与最终 System 基线拼接。
            llm_client: 需满足 ``LLMClient`` 协议所要求的方法集。
            wash_client: 可选廉价 OpenAI 兼容客户端，用于工具结果 Wash 压缩；缺省时回落到 ``llm_client``。
            router_client: 可选路由探针专用客户端（lifespan 内单例）；缺省时路由探针回落到 ``llm_client``（每请求 Working）。
            max_turns: 最大 ReAct 轮次。
            allowed_tools: 可执行工具白名单；为 None 时不做限制。
            agent_config: 工具超时与 wash 策略；缺省使用内置 ``OligoAgentConfig()``。
            persona: L2 可选，Final Stream 以 ``[PERSONA OVERRIDE]`` 注入；与 ``system_core`` 去空白后相同时跳过 L2。
            authors_note: L3 可选，以 ``[AUTHOR'S NOTE]`` 置于最后注入。
            metrics_service: 可选；若提供则记录工具调用延迟与成败（Oligo HTTP 层注入）。
        """
        self._system_core = system_core
        self._persona = persona
        self._authors_note = authors_note
        self._skill_override = (skill_override or "").strip() or None
        self.allowed_tools = allowed_tools
        self._agent_config = agent_config or OligoAgentConfig()

        self.raw_messages: list[ChatMessage] = _ensure_chat_messages(raw_messages)
        self.llm_client = llm_client
        self.wash_client = wash_client
        self._router_client: LLMClient = router_client or llm_client
        self.max_turns = max_turns
        self._metrics_service = metrics_service
        self._current_turn: int = 0
        self._phase: AgentPhase = AgentPhase.ROUTING
        # A.1: conversation-level identity (default_factory generates UUID when session_id absent)
        self._conversation_ctx = (
            ConversationContext(session_id=session_id)
            if session_id is not None
            else ConversationContext()
        )

        # FC.2a: per-request artifact accumulator. Populated after each turn's wash;
        # emitted once as `bb-message-artifacts` before the success-path return.
        # Never enters self.messages or any LLM payload.
        self._session_artifacts: list[Artifact] = []
        self._artifact_keys: set[tuple[str, str]] = set()

        router_body = self._build_router_system_prompt()

        self.messages: list[ChatMessage] = [
            ChatMessage(role="system", content=router_body),
            *[m.model_copy(deep=True) for m in self.raw_messages],
        ]

    def _compute_active_router_components(self) -> set[str]:
        """决定 Router system 中启用的 ``PromptComponent`` id（含 ``dynamic_timestamp``）。

        ``skill`` 类条件与 MW.0 一致；persona / authors 仅由
        :meth:`_compute_active_final_components` 控制，不属于 Router system。
        """
        ids = {
            "router_core",
            "router_tool_registry",
            "dynamic_timestamp",
        }
        if self._skill_override:
            ids.add("router_skill_directive")
        return ids

    def _compute_active_final_components(self) -> set[str]:
        """决定 Final system 中启用的 fragment id（含 ``dynamic_timestamp``）。"""
        ids = {
            "final_system_core",
            "final_guardrail",
            "dynamic_timestamp",
        }
        if self._skill_override:
            ids.add("final_skill_directive")
        if self._persona and self._persona.strip() != self._system_core.strip():
            ids.add("final_persona_override")
        if self._authors_note:
            ids.add("final_authors_note")
        return ids

    def _prompt_context(
        self, tool_list_max_chars: int | None = None
    ) -> dict[str, Any]:
        """供 ``PromptComposer.compose`` 使用的统一上下文（未使用的键由非活动组件忽略）。"""
        return {
            "system_core": self._system_core,
            "skill_override": self._skill_override or "",
            "persona": self._persona or "",
            "authors_note": self._authors_note or "",
            "tool_list": self._build_tool_list_text(max_chars=tool_list_max_chars),
            "timestamp": datetime.now().isoformat(),
        }

    def _build_tool_list_text(self, max_chars: int | None = None) -> str:
        """按 ToolRegistry 元数据与 ``allowed_tools`` 生成 router 工具块。"""
        return _render_tool_list(self.allowed_tools, max_chars=max_chars)

    def _build_router_system_prompt(self) -> str:
        """从 ``PromptComposer`` 组装路由 System 文案。"""
        composer = get_prompt_composer()
        active_ids = self._compute_active_router_components()
        context = self._prompt_context()
        stable, dynamic = composer.compose(
            stage=PromptStage.ROUTER,
            context=context,
            active_ids=active_ids,
        )
        body = f"{stable}\n\n{dynamic}".strip()
        logger.debug(
            "[Prompt] router compose stable_len=%s dynamic_len=%s total_len=%s",
            len(stable),
            len(dynamic),
            len(body),
        )
        return body

    def _build_router_continuation_prompt(self) -> str:
        return _load_md("router_continuation.md.j2")

    def _apply_history_sanitizer_to_messages(self) -> None:
        """位点 C：在送往 LLM 前清洗 ``self.messages`` 历史（层 3）。"""
        self.messages = TextSanitizer.sanitize_messages_history(self.messages)  # type: ignore[assignment]

    def _final_persona_system_content(self) -> str:
        """
        构建 Final Stream 的 System Prompt：L1（core + 技能）> L2（persona）> L3（作者注）>
        guardrail + 动态时间戳；禁止 Final 输出 ``<CMD:...>`` 的约束在 ``final_guardrail`` 片段中。
        """
        composer = get_prompt_composer()
        context = self._prompt_context()
        active_ids = self._compute_active_final_components()
        stable, dynamic = composer.compose(
            stage=PromptStage.FINAL,
            context=context,
            active_ids=active_ids,
        )
        logger.debug(
            "[Prompt] final compose stable_len=%s dynamic_len=%s",
            len(stable),
            len(dynamic),
        )
        return f"{stable}\n\n{dynamic}".strip()

    def _parse_tool_args(self, raw_args: str) -> dict[str, Any]:
        """Parse tool args（含 TP.3 保守修复）；返回值不含 repairs 列表。"""
        args, _ = parse_args_with_repair(raw_args)
        return args

    def _parse_tool_calls(self, probe_response: str) -> list[PlannedToolCall]:
        """
        Parse ``<tool_call>``（XML）与 ``<CMD:...>`` 为结构化计划并完成 allowlist 解析。

        匹配前先去掉 Markdown 代码块/行内代码，避免「举例」中的工具标签被执行（S0.4 Fix A）。

        参数须 parse 为合法 JSON object；失败则 ``allowed=False``（S0.4 Fix B）。
        """
        planned: list[PlannedToolCall] = []
        for_matching = _strip_markdown_code_for_cmd_extraction(probe_response)
        for pc in parse_tool_calls_unified(for_matching):
            tool_name = pc.tool_name
            raw_args = pc.raw_args
            plan_id = planned_call_id_from_parsed(pc)
            if not re.fullmatch(r"[a-zA-Z0-9_]+", tool_name):
                planned.append(
                    PlannedToolCall(
                        id=plan_id,
                        tool_name=tool_name,
                        raw_args=raw_args,
                        args={},
                        allowed=False,
                        deny_reason=(
                            f"Malformed tool name {tool_name!r}; "
                            "expected one token [a-zA-Z0-9_]+"
                        ),
                        repairs_applied=[],
                    )
                )
                continue
            try:
                args, repairs_applied = parse_args_with_repair(raw_args)
            except ValueError as e:
                logger.warning(
                    "[Oligo] Parser: tool args parse failed for tool=%s (%s): %s; "
                    "denying call (invalid args JSON).",
                    tool_name,
                    pc.source_format,
                    e,
                )
                planned.append(
                    PlannedToolCall(
                        id=plan_id,
                        tool_name=tool_name,
                        raw_args=raw_args,
                        args={},
                        allowed=False,
                        deny_reason=(
                            f"Invalid JSON in tool args for tool '{tool_name}': {e}"
                        ),
                        repairs_applied=[],
                    )
                )
                continue
            if repairs_applied:
                logger.info(
                    "[Tool] Args repaired for %s: %s",
                    tool_name,
                    repairs_applied,
                )
            if tool_name not in TOOL_REGISTRY:
                allowed = False
                deny_reason = (
                    f"Tool '{tool_name}' is not a registered tool. "
                    f"Available: {list(TOOL_REGISTRY.keys())}"
                )
            elif self.allowed_tools is None:
                allowed = True
                deny_reason = None
            elif tool_name in self.allowed_tools:
                allowed = True
                deny_reason = None
            else:
                allowed = False
                deny_reason = (
                    f"Tool '{tool_name}' is not allowed under current skill."
                )
            planned.append(
                PlannedToolCall(
                    id=plan_id,
                    tool_name=tool_name,
                    raw_args=raw_args,
                    args=args,
                    allowed=allowed,
                    deny_reason=deny_reason,
                    repairs_applied=repairs_applied,
                )
            )
        return planned

    def _maybe_record_tool_call(
        self, tool_name: str, success: bool, latency_ms: float
    ) -> None:
        svc = self._metrics_service
        if svc is not None:
            svc.record_tool_call(tool_name, success, latency_ms)

    def _maybe_record_wash(
        self, original_length: int, washed_length: int, tool_name: str
    ) -> None:
        svc = self._metrics_service
        if svc is not None:
            svc.record_wash(original_length, washed_length, tool_name)

    async def _execute_tool(
        self, tool_name: str, args: dict[str, Any]
    ) -> tuple[str, list[Artifact] | None]:
        """
        从 ``TOOL_REGISTRY`` 调度工具执行；入参为 planning 阶段已解析的 dict。

        Returns:
            ``(text, artifacts)``。``text`` 是 LLM 面向字符串（与历史 ``str`` 返回等价）；
            ``artifacts`` 仅当工具返回 ``ToolOutput`` 且其 ``artifacts`` 非空时为 ``list``,
            否则为 ``None``。错误路径返回 ``("Error: ...", None)``。
        """
        if tool_name not in TOOL_REGISTRY:
            out = f"Error: Tool '{tool_name}' is not recognized by the Chimera OS."
            logger.info(
                "[Tool] X-RAY Tool '%s' returned (first 300 chars): %r",
                tool_name,
                out[:300],
            )
            return out, None
        fn = TOOL_REGISTRY[tool_name]
        artifacts: list[Artifact] | None = None
        try:
            result = await fn(**args)
            if isinstance(result, ToolOutput):
                out = result.text
                artifacts = result.artifacts or None
            else:
                out = str(result)
        except TypeError as e:
            out = f"Error: Tool '{tool_name}' invalid args: {e}"
        logger.info(
            "[Tool] X-RAY Tool '%s' returned (first 300 chars): %r",
            tool_name,
            out[:300],
        )
        return out, artifacts

    async def _execute_tool_with_deadline(
        self, tool_name: str, args: dict[str, Any]
    ) -> tuple[str, list[Artifact] | None]:
        """调度层死线：不修改 `_execute_tool`，仅包裹 `wait_for`。"""
        deadline = self._agent_config.tool_execution_deadline_seconds
        try:
            return await asyncio.wait_for(
                self._execute_tool(tool_name, args),
                timeout=deadline,
            )
        except asyncio.TimeoutError:
            logger.warning(
                "[Tool] Tool deadline exceeded (%.0fs): tool=%s",
                deadline,
                tool_name,
            )
            return (
                f"[TOOL TIMEOUT]: Execution exceeded {deadline}s and was terminated.",
                None,
            )

    @staticmethod
    def _split_planned_denied_allowed(
        planned_calls: list[PlannedToolCall],
    ) -> tuple[list[ExecutedToolResult], list[PlannedToolCall]]:
        """拒绝项物化为 ``ExecutedToolResult``；允许项保留为计划列表（顺序不变）。"""
        denied_out: list[ExecutedToolResult] = []
        allowed_plans: list[PlannedToolCall] = []
        for plan in planned_calls:
            if not plan.allowed:
                dr = plan.deny_reason
                if dr is None:
                    dr = "Tool invocation denied."
                denied_out.append(
                    ExecutedToolResult(
                        call_id=plan.id,
                        tool_name=plan.tool_name,
                        args=plan.args,
                        status=ToolCallStatus.DENIED,
                        raw_result=f"[Permission Denied] {dr}",
                        washed_result=None,
                        error_message=dr,
                        elapsed_ms=None,
                    )
                )
            else:
                allowed_plans.append(plan)
        return denied_out, allowed_plans

    async def _execute_tool_plan_batch(
        self,
        batch: list[PlannedToolCall],
        emit_tool_sse: Callable[[str], Awaitable[None]] | None = None,
    ) -> list[ExecutedToolResult]:
        """执行单批计划：单工具直接 await；多工具 ``asyncio.gather``（与历史异常语义一致）。

        ``emit_tool_sse``：可选；每条为完整 SSE 文本帧（如 ``sse_event("bb-tool-start", ...)``），
        仅在工具进入队列前后发出（IR.3 会话级遥测；不持久化）。
        """
        if emit_tool_sse:
            for plan in batch:
                await emit_tool_sse(
                    sse_event(
                        "bb-tool-start",
                        {
                            "call_id": plan.id,
                            "tool_name": plan.tool_name,
                            "started_at_ms": int(time.time() * 1000),
                        },
                    )
                )

        async def _run_one(plan: PlannedToolCall) -> ExecutedToolResult:
            t0 = time.perf_counter()
            try:
                raw, artifacts = await self._execute_tool_with_deadline(
                    plan.tool_name, plan.args
                )
            except asyncio.CancelledError:
                raise
            except CLIENT_GONE_EXCEPTIONS:
                raise
            except Exception as e:
                elapsed_ms = (time.perf_counter() - t0) * 1000.0
                self._maybe_record_tool_call(plan.tool_name, False, elapsed_ms)
                return ExecutedToolResult(
                    call_id=plan.id,
                    tool_name=plan.tool_name,
                    args=plan.args,
                    status=ToolCallStatus.ERROR,
                    raw_result=f"Error: {e}",
                    washed_result=None,
                    error_message=str(e),
                    elapsed_ms=int(elapsed_ms),
                )
            elapsed_ms = (time.perf_counter() - t0) * 1000.0
            if raw.startswith("[TOOL TIMEOUT]:"):
                self._maybe_record_tool_call(plan.tool_name, False, elapsed_ms)
                return ExecutedToolResult(
                    call_id=plan.id,
                    tool_name=plan.tool_name,
                    args=plan.args,
                    status=ToolCallStatus.TIMEOUT,
                    raw_result=raw,
                    washed_result=None,
                    error_message=raw,
                    elapsed_ms=int(elapsed_ms),
                )
            ok = not str(raw).startswith("Error:")
            self._maybe_record_tool_call(plan.tool_name, ok, elapsed_ms)
            return ExecutedToolResult(
                call_id=plan.id,
                tool_name=plan.tool_name,
                args=plan.args,
                status=ToolCallStatus.SUCCESS,
                raw_result=str(raw),
                washed_result=None,
                error_message=None,
                elapsed_ms=int(elapsed_ms),
                artifacts=artifacts,
            )

        if len(batch) == 1:
            executed_single = [await _run_one(batch[0])]
            if emit_tool_sse:
                for er in executed_single:
                    await emit_tool_sse(
                        sse_event(
                            "bb-tool-done",
                            {
                                "call_id": er.call_id,
                                "status": er.status.name,
                                "elapsed_ms": er.elapsed_ms
                                if er.elapsed_ms is not None
                                else 0,
                            },
                        )
                    )
            return executed_single

        results = await asyncio.gather(
            *(_run_one(p) for p in batch),
            return_exceptions=True,
        )
        executed: list[ExecutedToolResult] = []
        for plan, r in zip(batch, results):
            if isinstance(r, BaseException):
                if isinstance(r, CLIENT_GONE_EXCEPTIONS):
                    raise r
                if not isinstance(r, Exception):
                    raise r
                self._maybe_record_tool_call(plan.tool_name, False, 0.0)
                executed.append(
                    ExecutedToolResult(
                        call_id=plan.id,
                        tool_name=plan.tool_name,
                        args=plan.args,
                        status=ToolCallStatus.ERROR,
                        raw_result=f"Error: {r}",
                        washed_result=None,
                        error_message=str(r),
                        elapsed_ms=None,
                    )
                )
            else:
                executed.append(r)

        if emit_tool_sse:
            for er in executed:
                await emit_tool_sse(
                    sse_event(
                        "bb-tool-done",
                        {
                            "call_id": er.call_id,
                            "status": er.status.name,
                            "elapsed_ms": er.elapsed_ms
                            if er.elapsed_ms is not None
                            else 0,
                        },
                    )
                )

        return executed

    async def _execute_tool_calls(
        self,
        planned_calls: list[PlannedToolCall],
        emit_tool_sse: Callable[[str], Awaitable[None]] | None = None,
    ) -> list[ExecutedToolResult]:
        """允许的工具按 ``concurrency_safe`` 分批调度；拒绝项仅物化结果。

        ``emit_tool_sse``：IR.3 可选；每批工具 ``bb-tool-start`` / ``bb-tool-done`` SSE（会话内，不持久化）。
        ``tool-progress`` 未接；未来可在具柄工具内调用同一 callable 扩展。
        """
        denied_out, allowed_plans = self._split_planned_denied_allowed(
            planned_calls
        )

        if not allowed_plans:
            logger.info(
                "[Tool] skip (no allowed tools executed; denied-only or empty)"
            )
            return denied_out

        logger.info(
            "[Tool] begin partitioned allowed=%s tools=%s",
            len(allowed_plans),
            [p.tool_name for p in allowed_plans],
        )
        registry = get_tool_registry()
        executed_allowed: list[ExecutedToolResult] = []
        for batch in partition_tool_calls(allowed_plans, registry):
            if emit_tool_sse:
                await emit_tool_sse(
                    _sse_data(
                        _sys_telemetry_obj(
                            _tool_batch_start_payload(batch)
                        )
                    )
                )
            executed_allowed.extend(
                await self._execute_tool_plan_batch(
                    batch, emit_tool_sse=emit_tool_sse
                )
            )

        logger.info(
            "[Tool] done results=%s",
            len(allowed_plans),
        )
        return denied_out + executed_allowed

    def _wash_context_for_intent(self) -> str:
        """
        捕获「调用工具前」路由可见的对话上下文（不含当前 probe 的 assistant 条）。
        使用最近两轮 user/assistant 交互（至多 4 条），单条过长时截断。
        """
        blocks: list[str] = []
        for m in self.messages[1:]:
            if m.role not in ("user", "assistant"):
                continue
            text = (m.content or "").strip()
            if len(text) > _WASH_CONTEXT_PER_MSG_CAP:
                text = (
                    text[:_WASH_CONTEXT_PER_MSG_CAP]
                    + "\n...[truncated]"
                )
            blocks.append(f"{m.role.upper()}:\n{text}")
        if not blocks:
            return "(no prior user/assistant dialogue before this probe)"
        recent = blocks[-_WASH_CONTEXT_MAX_MESSAGES:]
        return "\n\n---\n\n".join(recent)

    async def _wash_tool_result(
        self,
        tool_name: str,
        tool_args: str,
        raw_result: str,
        context: str,
    ) -> str:
        """
        Intent-Driven Dynamic Wash：结合路由意图过滤工具噪声，始终经 Cognitive Filter（无字数门槛）。
        失败时降级为硬截断 + 后缀。
        """
        washer_sys = _load_md("wash_system_prompt.md").format(
            tool_name=tool_name,
            tool_args=tool_args,
            context=context,
            raw_result=raw_result,
        )
        compress_client = self.wash_client or self.llm_client
        x_len = len(raw_result)
        logger.info(
            "[Wash] intent_wash begin tool=%s raw_chars=%s backend=%s",
            tool_name,
            x_len,
            "wash_client" if self.wash_client is not None else "llm_client",
        )
        wash_messages: list[dict[str, str]] = [
            {"role": "system", "content": washer_sys},
            {
                "role": "user",
                "content": "Output the Cognitive Filter result only (no preamble).",
            },
        ]
        try:
            washed = await compress_client.generate_raw_text(wash_messages)
            out = str(washed)
            logger.info(
                "[Wash] ok %s chars -> %s chars", x_len, len(out)
            )
            self._maybe_record_wash(len(raw_result), len(out), tool_name)
            return out
        except CLIENT_GONE_EXCEPTIONS:
            raise
        except Exception:
            logger.warning(
                "[Wash] failed, degrading to hard truncation.",
            )
            degraded = raw_result[:_WASH_FALLBACK_CHARS] + _WASH_TRUNC_SUFFIX
            logger.info(
                "[Wash] degraded %s chars -> %s chars",
                x_len,
                len(degraded),
            )
            self._maybe_record_wash(len(raw_result), len(degraded), tool_name)
            return degraded

    def _accumulate_artifacts(
        self, results: list[ExecutedToolResult]
    ) -> None:
        """FC.2a: collect artifacts post-wash, deduped by ``(kind, path)``.

        Called once per turn after ``_wash_tool_results`` returns. Never touches
        ``self.messages``; the accumulator is emitted as a single
        ``bb-message-artifacts`` SSE frame before the success-path return.
        """
        for er in results:
            if not er.artifacts:
                continue
            for art in er.artifacts:
                key = (art.kind, art.path)
                if key in self._artifact_keys:
                    continue
                self._artifact_keys.add(key)
                self._session_artifacts.append(art)

    def archive_segment(self, start_idx: int, end_idx: int, reason: str) -> None:
        """Replace self.messages[start_idx:end_idx] with a tombstone; persist originals to audit log.

        start_idx must be >= 1 (slot 0 is the system message and is protected).
        """
        if start_idx < 1:
            raise ValueError("Cannot archive system slot (index 0).")
        if end_idx <= start_idx:
            raise ValueError("end_idx must be > start_idx.")
        segment = self.messages[start_idx:end_idx]
        if not segment:
            raise ValueError("No messages in the specified range.")

        from src.crucible.core.platform import get_chimera_root
        import json as _json

        archive_dir = get_chimera_root() / "archive_log"
        archive_dir.mkdir(parents=True, exist_ok=True)
        session_ts = datetime.now().strftime("%Y%m%dT%H%M%S%f")
        log_path = archive_dir / f"{session_ts}.jsonl"
        with log_path.open("w", encoding="utf-8") as f:
            for m in segment:
                f.write(_json.dumps(m.model_dump(), ensure_ascii=False) + "\n")

        tombstone = ChatMessage(
            role="user",
            content=f"[ARCHIVED] {reason}. Status: superseded. Do not reference.",
        )
        self.messages[start_idx:end_idx] = [tombstone]
        if not hasattr(self, "_archive_log"):
            self._archive_log: dict = {}
        tombstone_idx = start_idx
        self._archive_log[tombstone_idx] = (log_path, segment)

    def unarchive_segment(self, tombstone_idx: int) -> None:
        """Restore messages previously archived at tombstone_idx."""
        if not hasattr(self, "_archive_log") or tombstone_idx not in self._archive_log:
            raise KeyError(f"No archived segment at index {tombstone_idx}.")
        _log_path, original = self._archive_log.pop(tombstone_idx)
        self.messages[tombstone_idx : tombstone_idx + 1] = original

    async def _wash_tool_results(
        self,
        results: list[ExecutedToolResult],
        context: str,
    ) -> tuple[list[ExecutedToolResult], list[tuple[str, int]]]:
        """Apply minimal per-tool wash policy; LLM wash only when rules say so.

        Returns:
            Updated results and a list of (tool_name, raw_char_count) for each
            invocation that ran the LLM Cognitive Filter (true wash).
        """
        out: list[ExecutedToolResult] = []
        real_washes: list[tuple[str, int]] = []
        for er in results:
            raw_text = er.raw_result or ""
            can_llm_wash = (
                er.status == ToolCallStatus.SUCCESS and raw_text.strip() != ""
            )
            if not can_llm_wash:
                out.append(
                    er.model_copy(update={"washed_result": er.raw_result})
                )
                continue

            tool_name = er.tool_name
            cfg = self._agent_config
            if tool_name in cfg.bypass_wash_tools:
                washed = raw_text
            elif (
                tool_name in cfg.force_wash_tools
                and len(raw_text) >= cfg.wash_min_chars
            ):
                tool_args = json.dumps(er.args, ensure_ascii=False)
                raw_char_count = len(raw_text)
                washed = await self._wash_tool_result(
                    tool_name,
                    tool_args,
                    raw_text,
                    context,
                )
                real_washes.append((tool_name, raw_char_count))
            else:
                washed = raw_text

            out.append(er.model_copy(update={"washed_result": washed}))
        return out, real_washes

    def _latest_user_message_text(self) -> str:
        """最近一次 user 消息正文，用于 IR.2 空结果 reflection hint。"""
        for m in reversed(self.raw_messages):
            if m.role == "user":
                return (m.content or "").strip()
        return ""

    def _render_tool_results_for_llm(
        self,
        results: list[ExecutedToolResult],
    ) -> str:
        """Format executed tool rows into one stable user message (no LLM calls)."""
        parts: list[str] = ["[SYSTEM TOOL RESULTS]", ""]
        any_failed_display = False
        any_empty_result = False
        user_expects = _user_text_suggests_expectation(
            self._latest_user_message_text()
        )

        for er in results:
            payload = er.washed_result or er.raw_result or er.error_message
            if payload is None or str(payload).strip() == "":
                result_body = "[No content]"
            else:
                result_body = str(payload)

            disp, reason = _classify_render_outcome(er, result_body)
            if disp == "failed":
                any_failed_display = True
                if reason == _TR_REASON_EMPTY_RESULT:
                    any_empty_result = True

            parts.append(_format_one_tool_result_xml(er, disp, reason, result_body))
            parts.append("")

        hint_lines: list[str] = []
        if any_failed_display:
            hint_lines.append(_REFLECTION_HINT_FAILURE)
        if any_empty_result and user_expects:
            hint_lines.append(_REFLECTION_HINT_EMPTY)
        hint_lines = hint_lines[:3]
        if hint_lines:
            parts.append("\n".join(hint_lines))
            parts.append("")

        parts.extend(
            [
                "Instruction:",
                "Synthesize the results above. If sufficient evidence is present, produce the final answer.",
                "Do NOT call more tools unless the results are clearly insufficient.",
            ]
        )
        return "\n".join(parts)

    def _phase_event(self, phase: AgentPhase) -> str:
        self._phase = phase
        return sse_event("bb-phase-transition", {
            "phase": phase.value,
            "turn": self._current_turn,
            "timestamp_ms": int(time.time() * 1000),
        })

    async def _step_route(self) -> RouteResult:
        if self._current_turn > 1:
            self.messages[0] = ChatMessage(
                role="system",
                content=self._build_router_continuation_prompt(),
            )
        logger.info("[Router] probe_begin turn=%s/%s", self._current_turn, self.max_turns)
        if self.messages:
            preview = self.messages[0].content[:1000] + (
                "..." if len(self.messages[0].content) > 1000 else ""
            )
            logger.info("[Router] ROUTER SYS (first 1000 chars): %s", preview[:1000])
        self._apply_history_sanitizer_to_messages()
        api_messages = _messages_to_api(self.messages)
        probe_response = await asyncio.wait_for(
            self._router_client.generate_raw_text(api_messages),
            timeout=_THEATER_LLM_OUTER_TIMEOUT_S,
        )
        logger.info("[Router] Full response (probe): %s", probe_response)
        probe_response = TextSanitizer.strip_reasoning_tags(probe_response)
        planned_calls = self._parse_tool_calls(probe_response)
        logger.info("[Router] probe_end tool_calls=%s", len(planned_calls))
        wash_context = self._wash_context_for_intent() if planned_calls else None
        probe_for_cmd = _strip_markdown_code_for_cmd_extraction(probe_response)
        backfill_draft = TextSanitizer.strip_tool_syntax_in_visible(probe_response)
        is_trivial = _is_router_pass_or_trivial(backfill_draft)
        return RouteResult(
            probe_response=probe_response,
            planned_calls=planned_calls,
            wash_context=wash_context,
            probe_for_cmd=probe_for_cmd,
            is_trivial=is_trivial,
        )

    async def _step_execute(
        self,
        planned_calls: list[PlannedToolCall],
        emit_tool_sse: Callable[[str], Awaitable[None]] | None,
    ) -> ExecuteResult:
        executed_results = await self._execute_tool_calls(
            planned_calls, emit_tool_sse=emit_tool_sse
        )
        registry = get_tool_registry()
        patched: list[ExecutedToolResult] = []
        has_long_running = False
        for er in executed_results:
            raw = er.raw_result or ""
            if (
                registry.is_long_running(er.tool_name)
                and raw.startswith("[Task started]")
            ):
                m = re.search(r'[0-9a-f]{8}', raw)
                task_id = m.group(0) if m else None
                er = er.model_copy(update={"task_id": task_id})
                has_long_running = True
            patched.append(er)
        return ExecuteResult(executed_results=patched, has_long_running=has_long_running)

    async def _step_wash(
        self,
        executed_results: list[ExecutedToolResult],
        wash_context: str,
    ) -> tuple[list[ExecutedToolResult], list[str]]:
        washed, wash_events = await self._wash_tool_results(executed_results, wash_context)
        self._accumulate_artifacts(washed)
        sse_frames: list[str] = []
        if not wash_events:
            sse_frames.append(_sse_data(_sys_telemetry_obj({
                "stage": "wash",
                "content": "All tool results bypassed wash (under threshold).",
            })))
        for tool_name_w, raw_chars in wash_events:
            sse_frames.append(_sse_data(_sys_telemetry_obj({
                "stage": "wash",
                "content": f"{tool_name_w}: {raw_chars} chars",
                "tool_name": tool_name_w,
                "raw_chars": raw_chars,
            })))
        return washed, sse_frames

    def _step_render(
        self,
        executed_results: list[ExecutedToolResult],
        probe_for_cmd: str,
        turn_id: str,
    ) -> None:
        cmd_only = extract_tool_syntax_for_history(probe_for_cmd)
        _cmd_len = len(cmd_only)
        if _cmd_len > 8000:
            cmd_only = f"{cmd_only[:8000]}\n...[truncated {_cmd_len - 8000} chars]"
        self.messages.append(ChatMessage(role="assistant", content=cmd_only, turn_id=turn_id))
        logger.info("[Wash] aggregate tool_results=%s", len(executed_results))
        tool_result_message = self._render_tool_results_for_llm(executed_results)
        self.messages.append(ChatMessage(role="user", content=tool_result_message, turn_id=turn_id))

    async def _step_synthesize(
        self,
        probe_response: str,
        is_trivial: bool,
        turn_id: str,
    ) -> AsyncGenerator[str, None]:
        backfill_draft = TextSanitizer.strip_tool_syntax_in_visible(probe_response)
        if not is_trivial:
            self.messages.append(
                ChatMessage(role="assistant", content=backfill_draft, turn_id=turn_id)
            )
            logger.info(
                "[Router] probe_draft_backfill chars=%s (raw_len=%s)",
                len(backfill_draft),
                len(probe_response or ""),
            )
        yield _sse_data(_sys_telemetry_obj({
            "stage": "router",
            "decision": "pass",
            "content": (
                "Router decided no tools; draft backfilled for Final."
                if not is_trivial
                else "Router decided no tools are needed."
            ),
        }))
        logger.info("[Final] begin (persona bind + generate buffer)")
        self._apply_history_sanitizer_to_messages()
        final_system = ChatMessage(
            role="system",
            content=self._final_persona_system_content(),
        )
        tail = [m.model_copy(deep=True) for m in self.messages[1:]]
        final_messages: list[ChatMessage] = [final_system, *tail]
        fs_preview = final_system.content[:1000] + (
            "..." if len(final_system.content) > 1000 else ""
        )
        logger.info("[Final] FINAL PERSONA SYS (first 150 chars): %s", fs_preview[:150])
        try:
            full_response = await asyncio.wait_for(
                self.llm_client.generate_raw_text(_messages_to_api(final_messages)),
                timeout=_THEATER_LLM_OUTER_TIMEOUT_S,
            )
        except asyncio.TimeoutError:
            logger.error(
                "[Final] LLM gateway timeout (final stream buffer, %.0fs watchdog)",
                _THEATER_LLM_OUTER_TIMEOUT_S,
            )
            yield sse_event("bb-stream-done", {"error": True, "message": "LLM gateway timeout"})
            return
        logger.info("[Final] Full response (final stream): %s", full_response)
        logger.info("[Final] buffer_ready chars=%s sse_chunking", len(full_response))
        yield _sse_data(_sys_telemetry_obj({"stage": "final", "content": "Generating final response…"}))
        stream_body = TextSanitizer.strip_tool_syntax_in_visible(
            TextSanitizer.strip_reasoning_tags(full_response)
        )
        chunk_size = 3
        for i in range(0, len(stream_body), chunk_size):
            yield _sse_chunk(stream_body[i : i + chunk_size])
            await asyncio.sleep(0.04)
        if self._session_artifacts:
            yield sse_event(
                "bb-message-artifacts",
                {"artifacts": [a.model_dump() for a in self._session_artifacts]},
            )
        logger.debug("[Oligo] Theater concluded on turn %s.", self._current_turn)

    async def _run_theater_stream(self) -> AsyncGenerator[str, None]:
        """Core theater loop; client-gone and pipe-broken handling live in ``run_theater`` only."""
        turn = 0

        while turn < self.max_turns:
            turn += 1
            self._current_turn = turn
            _turn_ctx = TurnContext(
                turn_id=TurnId.create(self._conversation_ctx.session_id, turn),
                turn_number=turn,
            )
            logger.debug(f"[Oligo] Theater turn {turn}/{self.max_turns}")
            turn_id_str = str(_turn_ctx.turn_id)

            yield self._phase_event(AgentPhase.ROUTING)
            try:
                route = await self._step_route()
            except asyncio.TimeoutError:
                logger.error(
                    "[Router] LLM gateway timeout (router probe, %.0fs watchdog)",
                    _THEATER_LLM_OUTER_TIMEOUT_S,
                )
                yield sse_event("bb-stream-done", {"error": True, "message": "LLM gateway timeout"})
                return

            if route.planned_calls:
                yield _sse_data(_sys_telemetry_obj({
                    "stage": "router",
                    "content": f"{len(route.planned_calls)} tool calls planned",
                    "decision": "parallel",
                    "parallel_count": len(route.planned_calls),
                }))
                for plan in route.planned_calls:
                    logger.info("[Router] Intercepted Raw Args from LLM: %r", plan.raw_args)
                    tel_router: dict[str, Any] = {
                        "stage": "router",
                        "content": f"Planning {plan.tool_name}",
                        "tool_name": plan.tool_name,
                        "decision": plan.tool_name,
                        "raw_args": plan.raw_args,
                    }
                    if plan.repairs_applied:
                        tel_router["args_repaired"] = True
                        tel_router["repairs_applied"] = list(plan.repairs_applied)
                    yield _sse_data(_sys_telemetry_obj(tel_router))
                    if not plan.allowed:
                        yield _sse_data(_sys_telemetry_obj({
                            "stage": "tool",
                            "content": f"Denied: {plan.tool_name}",
                            "tool_name": plan.tool_name,
                            "deny_reason": (plan.deny_reason or ""),
                        }))

                tool_sse_q: asyncio.Queue[str | None] = asyncio.Queue()

                async def emit_tool_sse(frame: str) -> None:
                    await tool_sse_q.put(frame)

                async def _run_execute(
                    _calls: list[PlannedToolCall] = route.planned_calls,
                    _emit: Callable[[str], Awaitable[None]] = emit_tool_sse,
                ) -> ExecuteResult:
                    try:
                        return await self._step_execute(_calls, _emit)
                    finally:
                        await tool_sse_q.put(None)

                yield self._phase_event(AgentPhase.EXECUTING)
                execute_task = asyncio.create_task(_run_execute())
                while True:
                    item = await tool_sse_q.get()
                    if item is None:
                        break
                    yield item
                execute_result = await execute_task

                for er in execute_result.executed_results:
                    yield _sse_data(_sys_telemetry_obj({
                        "stage": "tool",
                        "content": f"{er.tool_name} → {er.status.name}",
                        "tool_name": er.tool_name,
                        "execution_status": er.status.name,
                    }))

                yield self._phase_event(AgentPhase.WASHING)

                if execute_result.has_long_running:
                    yield self._phase_event(AgentPhase.AWAITING_TASK)
                    svc = get_task_service()
                    lr_ers = [er for er in execute_result.executed_results if er.task_id]
                    events = await asyncio.gather(
                        *[svc.await_completion(er.task_id) for er in lr_ers],
                        return_exceptions=True,
                    )
                    event_by_call = {er.call_id: ev for er, ev in zip(lr_ers, events)}
                    patched_ers: list[ExecutedToolResult] = []
                    for er in execute_result.executed_results:
                        ev = event_by_call.get(er.call_id)
                        if ev is None:
                            patched_ers.append(er)
                        elif isinstance(ev, BaseException):
                            patched_ers.append(er.model_copy(update={
                                "status": ToolCallStatus.ERROR,
                                "error_message": f"await failed: {ev}",
                                "washed_result": f"[Task await failed] {ev}",
                            }))
                        elif ev.event_type == TaskEventType.COMPLETED:
                            patched_ers.append(er.model_copy(update={
                                "washed_result": ev.message or "",
                                "raw_result": ev.message or "",
                                "status": ToolCallStatus.SUCCESS,
                            }))
                        else:
                            patched_ers.append(er.model_copy(update={
                                "status": ToolCallStatus.ERROR,
                                "error_message": ev.error or "task failed",
                                "washed_result": f"[Task failed] {ev.error or ''}",
                            }))
                    washed, wash_frames = await self._step_wash(
                        patched_ers, route.wash_context or ""
                    )
                    for frame in wash_frames:
                        yield frame
                    self._step_render(washed, route.probe_for_cmd or "", turn_id_str)
                    continue

                washed, wash_frames = await self._step_wash(
                    execute_result.executed_results, route.wash_context or ""
                )
                for frame in wash_frames:
                    yield frame

                yield self._phase_event(AgentPhase.RENDERING)
                self._step_render(washed, route.probe_for_cmd or "", turn_id_str)
                continue

            yield self._phase_event(AgentPhase.SYNTHESIZING)
            async for chunk in self._step_synthesize(
                route.probe_response, route.is_trivial, turn_id_str
            ):
                yield chunk
            return

        error_msg = "\n\n[SYSTEM FATAL]: Agent exhausted max turns. Shutting down."
        logger.error("[Oligo] Fallback: %s", error_msg)
        yield sse_event("bb-stream-done", {"error": True, "message": error_msg.strip()})

    async def run_theater(self) -> AsyncGenerator[str, None]:
        """
        剧场版主循环：闭门思考 → 检查 CMD → 终极推流。

        客户端断连类异常仅在**本方法**最外层统一记录并下发 ``bb-stream-done`` 信标；
        内层 ``_run_theater_stream`` 与各 helper 对 ``CLIENT_GONE_EXCEPTIONS`` 一律原样上抛。
        """
        try:
            async for chunk in self._run_theater_stream():
                yield chunk
        except CLIENT_GONE_EXCEPTIONS:
            _handle_client_gone()
            yield sse_event("bb-stream-done", {"aborted": True, "reason": "client_gone"})
            return
        except Exception as exc:
            if _looks_like_pipe_broken(exc):
                _handle_client_gone()
                yield sse_event("bb-stream-done", {"aborted": True, "reason": "client_gone"})
                return
            raise

    async def fork_subagent(
        self,
        prompt: str,
        system_core: str | None = None,
        max_turns: int = 3,
    ) -> str:
        child_max_turns = min(max_turns, self.max_turns - self._current_turn)
        child = ChimeraAgent(
            raw_messages=[{"role": "user", "content": prompt}],
            system_core=system_core or self._system_core,
            skill_override=None,
            llm_client=self.llm_client,
            wash_client=self.wash_client,
            router_client=self._router_client,
            max_turns=child_max_turns,
            allowed_tools=self.allowed_tools,
            agent_config=self._agent_config,
        )
        chunks: list[str] = []
        async for chunk in child._run_theater_stream():
            chunks.append(chunk)
        raw = "".join(chunks)
        # strip SSE framing, keep only text content
        lines = [
            line.removeprefix("data: ").strip()
            for line in raw.splitlines()
            if line.startswith("data: ") and '"text"' in line
        ]
        summary = " ".join(lines)
        return summary[:4096]


async def run_isolated(
    prompt: str,
    system_core: str,
    llm_client: LLMClient,
    wash_client: LLMClient | None = None,
    router_client: LLMClient | None = None,
    max_turns: int = 3,
    allowed_tools: list[str] | None = None,
) -> str:
    """Standalone fork for tool-layer callers that have no parent agent."""
    agent = ChimeraAgent(
        raw_messages=[{"role": "user", "content": prompt}],
        system_core=system_core,
        skill_override=None,
        llm_client=llm_client,
        wash_client=wash_client,
        router_client=router_client,
        max_turns=max_turns,
        allowed_tools=allowed_tools,
    )
    chunks: list[str] = []
    async for chunk in agent._run_theater_stream():
        chunks.append(chunk)
    raw = "".join(chunks)
    lines = [
        line.removeprefix("data: ").strip()
        for line in raw.splitlines()
        if line.startswith("data: ") and '"text"' in line
    ]
    summary = " ".join(lines)
    return summary[:4096]


# --- TEST HARNESS ---
if __name__ == "__main__":
    import asyncio

    from pydantic import BaseModel

    from src.oligo.tools.vault_tools import set_vault_adapter

    class _HarnessVault:
        async def search_notes(self, query: str, top_k: int = 3) -> str:
            return f"[HarnessVault] snippets for {query!r}"

        async def search_by_attribute(self, key: str, value: str, top_k: int = 5) -> str:
            return f"[HarnessVault] attr {key}={value!r}"

        async def query_graph(
            self,
            node_type: str | None = None,
            link_pattern: str | None = None,
            max_depth: int = 2,
        ) -> list[dict[str, Any]]:
            return []

    class MockLLMClient:
        """无 API 调用的测试客户端（与 ``LLMClient`` 结构兼容）。"""

        async def generate_raw_text(self, messages: list[dict[str, str]]) -> str:
            full_conv = " ".join(m.get("content", "") for m in messages)
            sys0 = messages[0].get("content", "") if messages else ""
            if "Chimera OS local router" in sys0:
                if "[SYSTEM TOOL RESULTS]" in full_conv:
                    return "Senpai, based on the vault: Titans is flawed. That is all."
                return '<CMD:search_vault({"query": "Titans"})> Searching...'
            if "[SYSTEM TOOL RESULTS]" in full_conv:
                return "Senpai, based on the vault: Titans is flawed. That is all."
            return "Hello from BB."

        async def stream_generate(
            self, messages: list[dict[str, str]]
        ) -> AsyncGenerator[str, None]:
            for c in "Senpai, Titans is flawed. That is all.":
                yield c
                await asyncio.sleep(0.03)

        async def generate_structured_data_async(
            self,
            system_prompt: str,
            user_prompt: str,
            response_model: type[BaseModel],
        ) -> BaseModel:
            raise NotImplementedError("MockLLMClient does not support structured output")

    async def test_run():
        set_vault_adapter(_HarnessVault())
        try:
            agent = ChimeraAgent(
                raw_messages=[{"role": "user", "content": "Fetch Titans."}],
                system_core="You are BB, a dramatic waifu persona.",
                skill_override=None,
                llm_client=MockLLMClient(),
            )
            print("Frontend receives:", end="", flush=True)
            async for chunk in agent.run_theater():
                print(chunk, end="", flush=True)
            print("\n\nDone.")
        finally:
            set_vault_adapter(None)

    asyncio.run(test_run())
