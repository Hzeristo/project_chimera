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
| EXT.2a | 移除 4000-char cap + budget-shrink loop | Pending |
| EXT.2b | 验证用户提供的 router_intro.md.j2 渲染正确; 更新 byte-lock | Content delivered |
| EXT.2c | 新增 router_continuation.md.j2 + theater loop 按 turn 切换 system prompt | Pending |
| EXT.2d | probe_response 解析前剥离 `<thinking>` 标签 | Pending |
| EXT.3 | 工具描述 rich化: ToolSpec 加 user_aliases / examples / common_mistakes | Pending |
| EXT.4 | Agentic theater讨论(架构决策, 不写代码) | Pending |

Dependencies: EXT.0 precedes all. EXT.1 precedes EXT.2a–2d.
EXT.2a → EXT.2b → EXT.2c → EXT.2d sequential. EXT.3 independent of EXT.2.
EXT.4 is a design discussion, not implementation.

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
  
- **Router as cognitive agent, Final as persona wrapper (ST 2026-05-27)**:
  Router is the cognitive subject of the entire turn — it understands intent,
  formulates retrieval strategy, evaluates results, and synthesizes answers.
  Final stage is a thin persona-application layer that rewrites Router's
  synthesis in the active persona's voice. Router does the thinking; Final
  does the styling.

- **First-turn vs continuation system prompt separation (ST 2026-05-27)**:
  Turn 1 injects the full router_intro.md.j2 (identity + intent framework +
  tool list + syntax + constraints + examples, ~1650 tokens static).
  Turn 2+ injects only router_continuation.md.j2 (~200 tokens): a focused
  evaluation-and-next-step directive. This prevents attention dilution and
  saves ~1400 tokens per subsequent turn. Implementation: agent.py theater
  loop selects system prompt by `turn == 0` vs `turn > 0`.

- **`<thinking>` tags for Chain-of-Thought (ST 2026-05-27)**:
  Router may output `<thinking>...</thinking>` blocks for internal reasoning
  before tool calls or natural language. These are stripped by TextSanitizer
  before reaching the user but logged for debugging. This is prompt-level CoT
  (Approach A), not API-level extended thinking. No provider-specific API
  changes needed. DeepSeek and Claude both respond well to this pattern.

- **Legacy CMD format removal (ST 2026-05-27)**:
  `<CMD:tool_name(args)>` format is removed from router prompt and will be
  removed from tool_protocol.py parser in debt week. Only XML `<tool_call>`
  format remains. Rationale: dual format dilutes attention, confuses non-Claude
  models, and adds unnecessary parsing branches.


## Out of Scope

- Half-blocking long task execution (EXT.4discussion, implementation in later phase)
- Final contamination hard filter (depends on EXT.4 architecture decisions)
- Memory CRUD → Phase IV
- Exocortex → Phase IV
