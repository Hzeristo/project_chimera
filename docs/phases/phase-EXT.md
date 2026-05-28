# Phase EXT — Prompt Externalization & Router Rewrite

**Status:** Active
**Sealed predecessor:** III.C
**Driving frictions:**
- friction-260526 E1 (router不识别"爬取papers" = daily_paper_pipeline)
- friction-260526 E2 (长任务启动后 Final幻觉工具调用 + 前端无进度)

## Mission

将Oligo 的 prompt 架构从"Python 内联常量"升级为"外部 Jinja2 模板 + 领域化 router prompt",
使 router具备意图识别能力,使 prompt迭代不再需要改Python 代码。

## Sprint Sequence

| Sprint | One-line goal | Status |
|---|---|---|
| EXT.0 | Audit: 当前 prompt 内联常量清单 +PromptComposer 渲染链路 | Sealed |
| EXT.1 | 外部化: 所有 inline prompt → .md/.md.j2 文件, 行为不变 | Sealed `10a282a` |
| EXT.2a | 结构脚手架: 移除 4000-char cap + router_intro.md.j2 占位段落头 | Pending |
| EXT.2b | Router prompt 内容填充: 用户提供的 5500+ token 模板粘贴 + 验证 | Content delivered |
| EXT.3 | 工具描述 rich化: ToolSpec 加 user_aliases / examples / common_mistakes | Pending |
| EXT.4 | Agentic theater讨论(架构决策, 不写代码) | Pending |

Dependencies: EXT.0 precedes all. EXT.1 precedes EXT.2/EXT.3.
EXT.2a precedes EXT.2b. EXT.4 is a design discussion, not implementation.

## Cross-Sprint Red Lines

- ❌ Do NOT introduce new Python dependencies (Jinja2 already present)
- ❌ Do NOT change PromptComposer's compose() core logic in EXT.1
- ❌ Do NOT modify agent.py theater loop in EXT.1-EXT.3
- ❌ Jinja2 (.md.j2) for static-at-registration templates only;
  runtime-variable templates use plain .md with Python str.format()

## Hard Sealing Conditions

1. Zero inline prompt constants remain in agent.py or prompt_composer.py
   (verified by Grep at seal time)
2. Router prompt contains intent classification framework with≥5 real
   usage examples per tool (verified by reading router template)
3. "爬取论文" as user input correctly triggers daily_paper_pipeline
   (verified by live test or mock)

## Design Decisions (from ST discussion, not re-derivable)

- **Two-phase rendering**: .md.j2 (Jinja2, registration-time) vs .md (str.format, compose-time).Avoids Jinja2/format brace conflict. See ST discussion2026-05-26.
- **Skill injection timing change**: Router gets skill one-line summary only;
  Final gets skill full text. Per Claude Code source analysis (Silver Bullet #1).
- **Router prompt structure**: 7 segments modeled after Anthropic's 16-segment
  architecture, adapted for single-user research OS. See ST discussion for
  segment breakdown.
- **Tool call format**: Keep `<tool_call name="..."><args>...</args></tool_call>`.
  Do NOT adopt Anthropic's ANTML `<function_calls><invoke>` format (provider-agnostic).

## Out of Scope

- Half-blocking long task execution (EXT.4discussion, implementation in later phase)
- Final contamination hard filter (depends on EXT.4 architecture decisions)
- Memory CRUD → Phase IV
- Exocortex → Phase IV
