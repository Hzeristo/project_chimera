# Phase Audit: Phase III.E — Oligo Orchestration Primitives

**Scope:** Read-only audit prerequisite for batch_planning of Phase III.E.
**Output location:** `docs/audits/III.E.0.md`
**Date:** 2026-06-11
**Mode:** Read-only — no fix proposals, no code modifications.

---

## Files read

| Path | Lines | Notes |
|---|---|---|
| `crucible_core/src/oligo/core/agent.py` | 1496 | full read |
| `crucible_core/src/oligo/api/server.py` | 224 | full read |
| `crucible_core/src/crucible/services/task_service.py` | 322 | full read |
| `crucible_core/src/crucible/core/schemas.py` | 679 | full read |
| `crucible_core/src/oligo/core/prompt_composer.py` | 405 | full read |
| `astrocyte/src-tauri/src/state.rs` | 101 | full read |
| `astrocyte/src-tauri/src/llm_client.rs` | 413 | full read |
| `astrocyte/src-tauri/src/task_stream.rs` | 76 | full read |

---

## Findings

| Q# | Driving sprint | Question | Answer | Evidence | Risk |
|---|---|---|---|---|---|
| Q1 | III.E.A | `ChimeraAgent` 构造函数接受哪些依赖注入参数？ | 11个参数：`raw_messages`, `system_core`, `skill_override`, `llm_client`, `wash_client`, `router_client`, `max_turns`, `allowed_tools`, `agent_config`, `persona`, `authors_note`, `metrics_service`。三个 LLM client 均为外部注入。 | `agent.py:381-395` | Low |
| Q1.a | III.E.A | 有没有现成的 `messages` 列表分离机制？ | 有。`raw_messages` (纯 user/assistant) 与 `self.messages` (含 system header) 是两个独立列表。`raw_messages` 在 `__init__` 赋值后不再被 theater loop 追加；所有追加都发生在 `self.messages`。fork_subagent 可直接传入一份 `raw_messages` 切片构造新 `ChimeraAgent`，重用 llm/wash 客户端实例。 | `agent.py:418`, `agent.py:433-436` | Low |
| Q2 | III.E.A | Theater loop 主 context 在哪里积累？ | `self.messages` 是唯一积累点。每轮工具调用追加两条：`ChatMessage(role="assistant", content=cmd_only)` 和 `ChatMessage(role="user", content=tool_result_message)`。无工具时可追加一条 `assistant` backfill。每轮调用后 `self.messages` 只增不减。 | `agent.py:1275-1289`, `agent.py:1298-1305` | Low |
| Q2.a | III.E.A | turn > 1 时 System message 如何处理？ | `self.messages[0]` 被原地替换为 `router_continuation.md.j2`（turn > 1 时覆写 router_intro）。只换 slot 0，不新增。 | `agent.py:1113-1117` | Low |
| Q3 | III.E.C | `_wash_tool_result` 如何消费 context window？有没有现成 segment 操作原语？ | `_wash_tool_result` 独立构造 `wash_messages: list[dict]`，不追加 `self.messages`，不消费主 context。`_wash_context_for_intent()` 从 `self.messages[1:]` 取最近 4 条 user/assistant 供 wash 参考，只读。目前无任何 segment 删除/替换/tombstone 操作原语——`self.messages` 只有 append。 | `agent.py:909-928`, `agent.py:955-983` | Low |
| Q4 | III.E.B | TaskService 事件总线结构是什么？ | `TaskService._subscribers: set[asyncio.Queue[TaskEvent]]`，`emit_event()` 广播到所有 subscriber；`subscribe()` 返回一个 `asyncio.Queue(maxsize=100)`；`/v1/tasks/stream` SSE 端点消费该 queue。Rust `task_stream.rs` 连接该端点并 `app.emit("bb-task-event", ...)` 转发到 webview。 | `task_service.py:63`, `task_service.py:93-103`, `server.py:183-218`, `task_stream.rs:10-76` | Low |
| Q4.a | III.E.B | subprocess stdout 流接入点在哪里？ | 目前无任何 subprocess 执行逻辑。TaskService 的 `run_task()` 接受 `Awaitable[str]`（Python 协程），不是外部进程。Claude Code babysitting 需要新增 `asyncio.create_subprocess_exec` + stdout 逐行读取 + 调用 `task_service.emit_event()` 的适配层。 | `task_service.py:297-309` | Med |
| Q5 | cross | Python `ChatMessage` 与 Rust `state::Message` 是否同构？ | **不完全同构**。Python `ChatMessage` 有 4 个字段：`role`, `content`, `tool_call_id`, `name`（后两者 optional）。Rust `state::Message` 只有 `role: String`, `content: String`——无 `tool_call_id`/`name`。Rust `OligoMessage`（发往 Oligo 的序列化体）同样只含 `role`/`content`。两者在 user/assistant 消息上等价，差异只在 OpenAI Function Calling 扩展字段（Chimera 未使用）。 | `schemas.py:282-298`, `state.rs:15-18`, `llm_client.rs:114-118` | Low |

---

## Cross-references discovered

- **`ChimeraAgent` 实例化路径**：由 `server.py:agent_invoke` 每请求构造一次（`server.py:111-124`），`llm_client` 来自 `build_openai_client_from_params`（每请求新建），`wash_client` / `router_client` 来自 lifespan 单例（`server.py:43-51`）。fork_subagent 可复用 lifespan 单例，仅新建 `messages` 列表。

- **`self.messages` 列表增长上限**：`max_turns`（默认来自 `settings.oligo.max_turns`）控制轮次上限。每轮最多追加 2 条（assistant cmd + user tool_result），Final 阶段不追加。上限 = `2 * max_turns` 条追加 + 初始 `1 + len(raw_messages)` 条。

- **`_session_artifacts` 独立于 `self.messages`**：`agent.py:428-429`，artifact 累加器从不进入 LLM payload，只在 success path 发出 SSE 帧。fork_subagent 自然需要自己的 artifact 累加器，不与父共享。

- **TaskService event bus 为纯 Python asyncio**：`_subscribers` 是 `set[asyncio.Queue]`，在 Uvicorn event loop 内全内存运行。subprocess stdout 流必须在同一 event loop 内用 `asyncio.create_subprocess_exec` 桥接——不能用 `subprocess.Popen`（会阻塞）。

---

## Notable cross-findings (no fix proposals — flagging for planning)

1. **`self.messages[0]` slot 约定是隐式合约。** Theater loop 在 turn > 1 时直接 `self.messages[0] = ChatMessage(...)` 覆写 system slot（`agent.py:1114`），假设 slot 0 永远是 system message。fork_subagent 构造新 `ChimeraAgent` 时 `__init__` 已写入 system slot（`agent.py:431-436`），该约定安全复用。但 archival 实现时若需操作 `self.messages` 的 slot 1+，需注意不能破坏 slot 0。

2. **`_wash_context_for_intent` 读取 `self.messages[1:]`，不读 `raw_messages`。** 意味着 wash context 会随每轮工具结果追加而增长（最多 4 条截断）。Archival 设计中若 tombstone 某条消息，需同步确认 wash context slice 不越界。此点影响 III.E.C 设计但不是阻碍。

3. **Rust `state::Message` 只是临时 in-memory 传输层。** Rust 侧 `session.history` (`state.rs:22-24`) 存放的是 `Vec<Message>` (role+content only)，序列化为 `OligoMessage` 发往 Oligo (`llm_client.rs:272-278`)。Oligo 收到后经 `_ensure_chat_messages` 重新包装为 `ChatMessage`。这条链路上没有 artifact、没有 tool_call_id——fork_subagent 只需处理 Python 侧，不需改 Rust。

4. **subprocess stdout 桥接无现成抽象。** `TaskService.run_task()` 包裹的是 Python 协程 (`task_service.py:297`)，不是外部进程。III.E.B 需在 `TaskService` 之上新增一个 `run_subprocess_task(task_id, cmd, args)` 适配方法，内部用 `asyncio.create_subprocess_exec` 并逐行 `emit_stage_progress`。**这是 III.E.B 唯一需要新建的基础设施**，其余（SSE 路由、Rust task_stream、前端 ActiveTaskPanel）全部已就绪。

---

## Audit complete

- 5 questions answered (+ 3 sub-questions)
- 22 file:line references
- 4 cross-references
- 4 notable cross-findings

**Suggested next:** `batch_planning` for Phase III.E.

---

*Generated by chimera-sprint-discipline phase_audit mode.*
