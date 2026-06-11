# Batch Plan: Phase III.E — Oligo Orchestration Primitives

**Output location:** `docs/plans/Phase-III.E-batch.md`
**Audit reference:** `docs/audits/III.E.0.md` (date: 2026-06-11)
**Phase doc:** `docs/phases/phase-III.E.md`
**Driving frictions:** friction-260526 (Entry 1 & 2)

This document is a single unit. User approves the whole sequence or rejects
the whole sequence. After approval, hand off to `chimera-code-taste`
batch_execution mode.

---

## Sprint Sequence

```
III.E.0 (audit — done) → III.E.A → III.E.C ┐
                                             ├→ III.E.B → (phase seal)
                         III.E.B ────────────┘
```

III.E.0 is the audit sprint (this document). A and C are context-management
family (do consecutively). B is subprocess-based (independent, can be last).
Dependencies stated in each sprint's "Predecessor assumptions" block.

---

## Sprint III.E.0: Audit

**Status:** ✅ Complete — `docs/audits/III.E.0.md`

---

## Sprint III.E.A: fork_subagent + run_isolated

**Friction reference:** friction-260526 Entry 2 (agent design 不合理：不白盒看不见进度条；需完整 agentic 逻辑框架)

**Predecessor assumptions:**
- III.E.0 audit complete — file:line evidence base confirmed. ✅

**Risk level:** 🔴 HIGH — new class + method, touches `agent.py` and `server.py`; no existing test coverage for subagent path.

### 目标
为 `ChimeraAgent` 添加 `fork_subagent(messages, prompt)` 工厂方法，在独立 `messages` 列表上运行隔离子任务，只向父返回文本 summary（≤1K tokens）。

### 设计要点(audit-derived)
- `ChimeraAgent.__init__` 接受 `raw_messages + system_core + llm_client + wash_client + router_client`，三个 LLM client 均为外部注入可复用 — audit ref: `agent.py:381-395`
- `raw_messages` 与 `self.messages` 是两个独立列表；fork 只需构造新 `ChimeraAgent` 传入隔离的 messages 切片 — audit ref: `agent.py:418`, `agent.py:433-436`
- lifespan 单例 `wash_client` / `router_client` 可在 fork 中直接复用（不是每请求新建）— audit ref: `server.py:43-51`
- `_session_artifacts` 独立于 `self.messages`，fork 自然隔离 — audit ref: `agent.py:428-429`
- Budget conservation (ST 2026-05-29): subagent `max_turns` ≤ parent remaining `max_turns - turn`；在 fork 时 assert，不是运行时框架。最小形式：`child_max_turns = min(requested, self.max_turns - self._current_turn)`，`_current_turn` 需在 theater loop 中跟踪。

### 任务范围
1. 在 `ChimeraAgent` 上添加 `async def fork_subagent(self, prompt: str, system_core: str | None = None, max_turns: int = 3) -> str` 方法 (`agent.py`, ~25 lines) — 内部构造新 `ChimeraAgent` 实例，复用 `self.wash_client` / `self._router_client`，运行 `run_theater()`，收集并截断 summary 返回
2. 添加 `run_isolated` 工具函数（或同文件 helper）以便 tool 层调用 fork (`agent.py` 或 `tools/`, ~15 lines)
3. Budget conservation guard (~6 lines): 在 `fork_subagent` 内计算 `child_max_turns = min(requested_turns, self.max_turns - self._current_turn)`；`self._current_turn` 在 `_run_theater_stream` loop 头部赋值（已有 `turn` 变量，expose 为实例属性即可，~2 lines）
4. 单元测试：验证 fork 后主 agent `self.messages` 长度不变；budget conservation 截断正确 (`tests/oligo/test_subagent.py`, ~35 lines)

### 验收
- fork 后父 `self.messages` 长度 == fork 前长度 — verifiable via `len(agent.messages)` assert in test
- fork 返回值为 `str`，长度 < 4096 chars (proxy for <1K tokens) — verifiable via `assert len(result) < 4096`
- `fork_subagent(max_turns=10)` on parent with 2 turns remaining → child `max_turns` == 2 — verifiable via unit test
- 满足 Hard Sealing Condition 1 (downgraded 2026-06-11): fork_subagent given a 50K-token prompt returns summary ≤ 4096 chars and parent messages does NOT contain the 50K content — verified by `test_hsc1_50k_prompt_does_not_enter_parent_messages`

### 红线
- ❌ NO arbitrary command execution tool (phase-wide)
- ❌ NO subagent nesting — `fork_subagent` 内的 `ChimeraAgent` 不得再调用 `fork_subagent`
- ❌ NO parallel subagent (sequential fork-merge only)
- ❌ Subagent 不得共享 `self.messages` / `_session_artifacts` — 必须各自独立
- ❌ 不进行机会主义重构

### 输出位置
- 代码: `crucible_core/src/oligo/core/agent.py`
- 测试: `crucible_core/tests/oligo/test_subagent.py`
- 文档: 推迟至 phase seal 统一更新

---

## Sprint III.E.C: Context Archival

**Friction reference:** ST 2026-05-28 (反复 rebuttal 中，失效的 proposal 污染后续推理)

**Predecessor assumptions:**
- III.E.A complete — `self.messages` 的结构与追加模式已确认不变。若 III.E.A 修改了 messages 结构需重新评估 slot 约定。

**Risk level:** 🟡 MED — touches `agent.py` message list; tombstone format is new but additive; ~20 lines.

### 目标
实现语义/意图驱动的 segment tombstone：用 `[ARCHIVED]` 占位符替换 `self.messages` 中指定 segment，原文写入 audit log，可 un-archive 恢复。

### 设计要点(audit-derived)
- `self.messages` slot 0 永远是 system message，archival 只操作 slot 1+，不得覆写 slot 0 — audit ref: `agent.py:1114`, cross-finding 1
- `_wash_context_for_intent` 读取 `self.messages[1:]` 最近 4 条；tombstone 替换后该 slice 仍有效（tombstone 是一条 ChatMessage，不删除）— audit ref: `agent.py:909-928`, cross-finding 2
- 无现有 segment 操作原语，全部新增 — audit ref: Q3

### 任务范围
1. 在 `ChimeraAgent` 上添加 `archive_segment(start_idx: int, end_idx: int, reason: str) -> None` 方法 (`agent.py`, ~20 lines) — 将 `self.messages[start_idx:end_idx]` 原文写入 `~/.chimera/archive_log/{session_ts}.jsonl`，替换为单条 tombstone `ChatMessage`
2. 添加 `unarchive_segment(tombstone_idx: int) -> None` 恢复方法 (`agent.py`, ~15 lines)
3. Tombstone 格式严格按 phase doc：`"[ARCHIVED] {summary}. Status: superseded. Do not reference."` — summary 由调用方提供
4. 单元测试：archive → messages 长度变化正确；unarchive → 原文恢复 (`tests/oligo/test_archival.py`, ~25 lines)

### 验收
- archive 后 `self.messages` 长度 = 原长度 - (end_idx - start_idx) + 1 — verifiable via len assert
- audit log 文件存在且包含原始 messages JSON — verifiable via file read in test
- un-archive 后 messages 恢复为原内容 — verifiable via content equality assert
- 满足 Hard Sealing Condition 2 (manual verify with live agent)

### 红线
- ❌ Archived segments 是 tombstoned，NOT deleted — 原文必须持久化到 audit log
- ❌ 不操作 `self.messages[0]` (system slot)
- ❌ 不触及 `raw_messages`
- ❌ 不进行机会主义重构

### 输出位置
- 代码: `crucible_core/src/oligo/core/agent.py`
- Audit log: `~/.chimera/archive_log/`
- 测试: `crucible_core/tests/oligo/test_archival.py`

---

## Sprint III.E.B: Claude Code Babysitting

**Friction reference:** friction-260526 Entry 2 (需要完整 agentic 逻辑框架；babysitting 是 Phase IV.C 前置)

**Predecessor assumptions:**
- None — independent of III.E.A and III.E.C (subprocess-based, not messages-based)
- TaskService event bus confirmed working: `task_service.py:63`, `server.py:183-218`, `task_stream.rs:10-76` ✅
- `TaskService.run_task()` 只接受 Python 协程，不支持外部进程 — needs new method — audit ref: Q4.a, cross-finding 4

**Risk level:** 🔴 HIGH — new subprocess infrastructure; stdout async bridging; exit-code failure detection; ~60 lines across 2 files.

### 目标
新增 `TaskService.run_subprocess_task(task_id, cmd, args)` 适配方法，用 `asyncio.create_subprocess_exec` 运行 Claude Code subprocess，逐行 stdout → `emit_stage_progress`，exit code != 0 时 `emit_failed`，stall 检测（N 秒无 stdout → timeout）。

### 设计要点(audit-derived)
- `TaskService.emit_stage_progress` / `emit_failed` / `emit_completed` 已存在且经 SSE 路由到 Rust `task_stream.rs` → `bb-task-event` → `ActiveTaskPanel` — audit ref: `task_service.py:143-167`, `task_stream.rs:37-67`
- Rust `task_stream.rs` 无需改动——已处理所有 `task-*` 事件 — audit ref: `task_stream.rs:38-67`
- 前端 `ActiveTaskPanel` 无需改动——消费已有 `bb-task-event` 事件
- 必须用 `asyncio.create_subprocess_exec`，不能用 `subprocess.Popen`（会阻塞 event loop）— audit ref: cross-finding 4
- DEAD_END lesson via backward trace (ST 2026-05-29): exit != 0 时调一次 `wash_client`（Haiku routing）做 backward trace — "Given this failure log, what was the EARLIEST decision that led to this failure? One sentence." 结果作为 lesson 字段。reuse `wash_client`，不新建 client。

### 任务范围
1. 在 `TaskService` 添加 `async def run_subprocess_task(self, task_id: str, cmd: list[str], stall_timeout_s: float = 300.0, wash_client: LLMClient | None = None) -> None` (`task_service.py`, ~40 lines) — `asyncio.create_subprocess_exec`, 逐行读 stdout, `emit_stage_progress`, exit != 0 → failure path
2. stall 检测：`asyncio.wait_for(proc.stdout.readline(), timeout=stall_timeout_s)` 超时 → `emit_failed(task_id, "stall: no stdout")`
3. 在 `task_service.py` 添加模块级 helper `async def _extract_failure_lesson(stdout_tail: str, wash_client: LLMClient) -> str` (~12 lines) — 单次 `generate_raw_text` backward trace 调用；失败时 fallback 到 `"lesson extraction failed"`
4. failure path: `lesson = await _extract_failure_lesson(stdout_tail, wash_client)` if `wash_client` else `""`; `emit_failed(task_id, f"exit {rc} | lesson: {lesson}")`
5. 单元测试：mock subprocess + mock `wash_client` 验证 emit 序列；lesson 出现在 `error` 字段；lesson 提取失败时降级不抛异常 (`tests/oligo/test_subprocess_task.py`, ~40 lines)

### 验收
- 正常退出(exit 0)：最终事件为 `COMPLETED` — verifiable via test
- 失败退出(exit != 0)：最终事件为 `FAILED`，`error` 字段含 exit code 和 one-sentence lesson — verifiable via test
- stall 超时：最终事件为 `FAILED`，`error` 含 "stall" — verifiable via test
- lesson 提取失败时降级为固定字符串，不抛异常 — verifiable via test with mock raising
- 满足 Hard Sealing Condition 3 (manual verify with deliberately failing Claude Code task)

### 红线
- ❌ 不新建 SSE 路由、不新建 Rust 代码、不新建前端组件——复用 Phase III.A Step 2 全部基础设施
- ❌ 不用 `subprocess.Popen`（阻塞 event loop）
- ❌ 不进行机会主义重构

### 输出位置
- 代码: `crucible_core/src/crucible/services/task_service.py`
- 测试: `crucible_core/tests/oligo/test_subprocess_task.py`

---

## Phase-wide Red Lines

- ❌ NO arbitrary command execution tool (no run_shell, no run_pwsh)
- ❌ NO subagent nesting (max depth 1)
- ❌ NO parallel subagent (sequential fork-merge only)
- ❌ NO interleaved-CoT via provider API
- ❌ Subagent shares NO mutable state with parent
- ❌ Archived segments tombstoned, NOT deleted
- ❌ Reuse existing TaskService event bus — do NOT build new infra
- ❌ All file operations stay within vault_root / project boundaries

---

## Hard Sealing Conditions (carried from phase doc)

1. Subagent isolation: deep-reading a 50K-token paper via fork_subagent increases main agent context by < 1K tokens — verified by token count
2. Context archival: an archived segment is replaced by tombstone in live context AND recoverable from audit log — verified by un-archive
3. Babysitting: invoking a Claude Code subprocess shows live progress in ActiveTaskPanel and produces a DEAD_END candidate node on failure — verified by live test with deliberately failing task

---

## Approval

User approves whole sequence or rejects whole sequence.

Upon approval, hand off to `chimera-code-taste` with:
> "Execute batch for Phase III.E per `docs/plans/Phase-III.E-batch.md`."

---

*Generated by chimera-sprint-discipline batch_planning mode.*
