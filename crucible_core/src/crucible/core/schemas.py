"""全系统唯一 Pydantic 数据字典（PaperMiner / Optics / Oligo / 工作流）。"""

from __future__ import annotations

# Re-exports for backward-compat: Oligo domain models live in oligo.core.schemas now.
# Downstream consumers that import from crucible.core.schemas continue to work.
from src.oligo.core.schemas import (  # noqa: F401
    AgentInvokeRequest,
    Artifact,
    ChatMessage,
    ExecutedToolResult,
    OligoAgentConfig,
    PlannedToolCall,
    PromptComponent,
    PromptRenderer,
    PromptStage,
    ToolCallStatus,
    ToolOutput,
)

from enum import Enum
from pathlib import Path
from typing import Any, Literal

from pydantic import BaseModel, ConfigDict, Field, model_validator

# --- Paper / Triage ---

SourceType = Literal[
    "arxiv_paper", "github_repo", "tech_blog", "book_chapter", "markdown"
]


class TaskEventType(str, Enum):
    CREATED = "created"
    STAGE_START = "stage_start"
    STAGE_PROGRESS = "stage_progress"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"


class PaperMetadata(BaseModel):
    """Typed metadata payload attached to a paper."""

    model_config = ConfigDict(extra="forbid")

    extracted_from: str | None = None
    score: int | None = None
    reason: str | None = None
    year: str | None = None
    authors: str | None = None
    extra_fields: dict[str, Any] = Field(default_factory=dict)


class Paper(BaseModel):
    """记录一篇 Paper 的信息"""

    id: str
    type: SourceType = Field(
        default="arxiv_paper", description="决定了 LLM 将以何种视角审视此文本"
    )
    title: str
    content_path: Path
    raw_text: str = Field(repr=False)
    year: str | None = Field(
        default=None,
        description="Official submission year from arXiv API when available.",
    )
    authors: str | None = Field(
        default=None,
        description="Official author list from arXiv API (comma-separated), when available.",
    )
    metadata: PaperMetadata = Field(default_factory=PaperMetadata)


class VerdictDecision(str, Enum):
    """Final decision labels for paper triage."""

    REJECT = "Reject"
    SKIM = "Skim"
    MUST_READ = "Must Read"


class PaperAnalysisResult(BaseModel):
    """Structured analysis result returned by LLM-based reviewer."""

    model_config = ConfigDict(extra="forbid")

    verdict: VerdictDecision = Field(
        description='Decision: "Reject" / "Skim" / "Must Read".'
    )
    short_moniker: str = Field(
        min_length=1,
        max_length=64,
        description=(
            "Must be EXACTLY ONE capitalized proper noun representing the core system/model name "
            "(e.g., 'HippoRAG', 'MemGPT', 'Titans'). DO NOT add descriptive words like 'Architecture', "
            "'Graph', or 'Memory'. If no distinct proper noun exists, invent a SINGLE capitalized "
            "portmanteau word. Exclude the raw paper ID or dates."
        ),
    )
    score: int = Field(
        ge=0,
        le=10,
        description="Overall score. Normal range is 1-10; 0 is reserved for degraded fallback.",
    )
    novelty_delta: str = Field(
        min_length=1, description="Compared with baseline, where is the gain?"
    )
    mechanism_summary: str = Field(min_length=1, description="Core mechanism summary.")
    critical_flaws: list[str] = Field(
        default_factory=list, description="Critical flaws and attack points."
    )
    baseline_models: list[str] = Field(
        default_factory=list,
        description=(
            "Models used as baselines (e.g., ['GPT-4', 'Llama-3-8B']). "
            "Empty list if none."
        ),
    )
    evaluation_datasets: list[str] = Field(
        default_factory=list,
        description=(
            "Benchmarks/Datasets tested on (e.g., ['MMLU', 'WebArena']). "
            "Empty list if none."
        ),
    )
    core_algorithm_steps: list[str] = Field(
        default_factory=list,
        description=(
            "Extremely concise step-by-step breakdown of the proposed "
            "mechanism/architecture. Empty list if none."
        ),
    )
    experimental_setup: str = Field(
        default="Not specified.",
        description=(
            "Forensic-grade execution pipeline for experiments: NOT a high-level summary. "
            "Dense bulleted report covering, when present or conspicuously absent: "
            "(1) Context ingestion—batch full-history stuffing vs true incremental/turn-by-turn; "
            "(2) Environment realism—mock static QA-style evaluation vs dynamic interactive envs; "
            "(3) Prompting hacks—oracle leakage, asymmetric few-shot vs baselines, forced CoT; "
            "(4) Memory state management—exact update mechanics (e.g. full recompute every N turns vs silent vector DB append). "
            "Use newline characters (\\n in JSON) between sections/bullets so downstream Markdown renders multiple lines; "
            "do not collapse into one uninterrupted paragraph unless the paper gives almost no detail."
        ),
    )
    ablation_findings: list[str] = Field(
        default_factory=list,
        description=(
            "Key takeaways from their ablation studies. What specific component did they remove or tweak, "
            "and how much did performance drop? (e.g., 'Removing the Time-aware Query Expansion caused a 15% drop in temporal reasoning accuracy.')"
        ),
    )


# --- Optics ---


class LensConfig(BaseModel):
    """单次并发 LLM 调用的透镜定义；`output_schema_name` 为可反射加载的受体类名。"""

    model_config = ConfigDict(extra="forbid", str_strip_whitespace=True)

    id: str = Field(..., min_length=1)
    system_prompt: str
    output_schema_name: str = Field(..., min_length=1)
    description: str


class MathArchExtraction(BaseModel):
    model_config = ConfigDict(extra="forbid", str_strip_whitespace=False)

    architecture_narrative: str = Field(
        default="",
        description=(
            "Design philosophy and data flow: 2–3 bullet lines each led by a bold key concept "
            "(per prompt)—not one wall of text."
        ),
    )
    core_equations: list[str] = Field(default_factory=list)
    pseudo_code: str = ""
    architecture_type: list[str] = Field(default_factory=list)


class EvalRigorExtraction(BaseModel):
    model_config = ConfigDict(extra="forbid", str_strip_whitespace=False)

    baselines: list[str] = Field(default_factory=list)
    datasets: list[str] = Field(default_factory=list)
    metrics_used: list[str] = Field(default_factory=list)
    ablation_target: str = Field(
        default="",
        description=(
            "Deep, critical analysis of ablations and empirical setup: what removing the core "
            "component does to metrics; use 2–3 bullet lines each led by a bold key concept per prompt."
        ),
    )


class MemoryPhysicsExtraction(BaseModel):
    model_config = ConfigDict(extra="forbid", str_strip_whitespace=False)

    mechanics_deep_dive: str = Field(
        default="",
        description=(
            "Extensive explanation of memory states, context bounds, overwrites; 2–3 bullet lines "
            "each led by a bold key concept per prompt—not one wall of text."
        ),
    )
    forgetting_mechanism: str = Field(
        default="",
        description="Concrete prose on forgetting / state handling (not a sparse label).",
    )
    context_window_tricks: str = Field(
        default="",
        description=(
            "Descriptive prose on context-window and state tricks (not a tag list); paragraph-style."
        ),
    )


class TaxonomyExtraction(BaseModel):
    model_config = ConfigDict(extra="forbid", str_strip_whitespace=False)

    classification_axes: list[str] = Field(
        default_factory=list,
        description="作者用来结构化学术领域的抽象维度（非模型枚举）；可短。",
    )
    core_categories: dict[str, str] = Field(
        default_factory=dict,
        description=(
            "键为类别名；值必须为 1–3 句高密度技术释义，说明在 LLM/Agent 语境下本文如何显式定义或实现该类别，"
            "以及与其它类别区分的架构边界（非目录短语）。"
        ),
    )


class ConsensusAndBottlenecks(BaseModel):
    model_config = ConfigDict(extra="forbid", str_strip_whitespace=False)

    major_limitations: list[str] = Field(
        default_factory=list,
        description="范式级通病与硬瓶颈；Prompt 要求每条高度概括且不少于约 20 字。",
    )


class FutureDirectionGap(BaseModel):
    model_config = ConfigDict(extra="forbid", str_strip_whitespace=False)

    direction: str = Field(
        ...,
        min_length=1,
        description="该研究坑位或方向的短名称（可对应 Future Work 小节，但须单独成条）。",
    )
    technical_void: str = Field(
        ...,
        min_length=1,
        description=(
            "指出现有理论与架构上仍未解决的具体断层：为何难、卡在哪（如可扩展性、可组合性、评测缺口），"
            "禁止仅堆砌 buzzword。"
        ),
    )


class StructuralGaps(BaseModel):
    model_config = ConfigDict(extra="forbid", str_strip_whitespace=False)

    future_directions: list[FutureDirectionGap] = Field(
        default_factory=list,
        description=(
            "每条为独立方向；须写清使该问题仍开放的架构或理论局限，而非复述小节标题。"
        ),
    )


class DeepReadAtlas(BaseModel):
    model_config = ConfigDict(extra="forbid", str_strip_whitespace=False)

    arxiv_id: str = Field(..., min_length=1)
    short_moniker: str = Field(..., min_length=1, max_length=64)
    title: str | None = None
    is_survey: bool = False

    math_arch: MathArchExtraction | None = None
    eval_rigor: EvalRigorExtraction | None = None
    memory_physics: MemoryPhysicsExtraction | None = None

    taxonomy: TaxonomyExtraction | None = None
    consensus_bottlenecks: ConsensusAndBottlenecks | None = None
    structural_gaps: StructuralGaps | None = None


# --- Oligo ---

# (ChatMessage migrated to oligo.core.schemas — re-exported above)

class SkillDefinition(BaseModel):
    """`~/.chimera/skills/{skill_id}.json` 的纯净定义（不含使用统计）。

    统计见 `~/.chimera/skill_stats.json`，由 `SkillStatsService` 维护。
    旧文件中若仍含 ``usage_count`` / ``success_rate`` / ``avg_tokens`` 等键，
    加载时会被忽略（``extra="ignore"``）。
    """

    model_config = ConfigDict(extra="ignore", str_strip_whitespace=True)

    id: str = Field(..., min_length=1)
    name: str = Field(..., min_length=1)
    description: str = ""
    system_override: str = Field(..., min_length=1)
    allowed_tools: list[str] | None = None
    category: str = Field(default="general", min_length=1)
    target_paper_type: list[str] = Field(default_factory=list)
    expected_output_format: str | None = None
    version: str = Field(default="1.0.0", min_length=1)
    last_updated: str | None = None


# (AgentInvokeRequest migrated to oligo.core.schemas — re-exported above)


# --- Oligo Tool Execution ---

# (ToolCallStatus, PlannedToolCall, Artifact, ToolOutput, ExecutedToolResult,
#  OligoAgentConfig migrated to oligo.core.schemas — re-exported above)


# --- Batch workflow ---


class BatchMustReadItem(BaseModel):
    score: int
    id: str
    paper_id: str
    short_moniker: str
    filename: str
    title: str
    novelty: str


class BatchFilterStats(BaseModel):
    total: int = 0
    must_read: int = 0
    skim: int = 0
    reject: int = 0
    errors: int = 0
    processed_ids: list[str] = Field(default_factory=list)
    must_read_titles: list[str] = Field(default_factory=list)
    must_read_items: list[BatchMustReadItem] = Field(default_factory=list)
    source_dir: Path | None = None


# --- Oligo PromptComposer ---

# (PromptStage, PromptRenderer, PromptComponent migrated to oligo.core.schemas — re-exported above)


class ToolSpec(BaseModel):
    """工具的元数据"""

    name: str = Field(description="工具名, 与 TOOL_REGISTRY 的 key 一致")
    description: str = Field(description="一行简介, 用于 router prompt")
    args_schema: dict[str, Any] = Field(
        default_factory=dict,
        description="JSON-schema-like 描述, 形如 {'query': {'type': 'str', 'required': True}}",
    )
    concurrency_safe: bool = Field(
        default=False,
        description="True=可与其他 concurrency_safe 工具并行. False=必须串行(可能改文件状态).",
    )
    long_running: bool = Field(
        default=False,
        description="True=立即返回 task_id 的异步工具(arxiv_miner / daily_paper_pipeline)",
    )
    examples: list[str] = Field(
        default_factory=list,
        description="可选: 1-2 个调用示例, 用于强化 router 的 in-context learning",
    )
    user_aliases: list[str] = Field(
        default_factory=list,
        description="用户可能使用的别名/中文表达, 用于 router intent matching",
    )
    common_mistakes: list[str] = Field(
        default_factory=list,
        description="常见误用提示, 仅在 verbose 渲染中展示",
    )

    model_config = ConfigDict(extra="forbid")


class TaskEvent(BaseModel):
    """task 状态变化事件, 通过 SSE 推送给前端。"""

    model_config = ConfigDict(extra="forbid")

    event_type: TaskEventType
    task_id: str
    task_type: str
    stage_id: str | None = Field(
        default=None,
        description="STAGE_START 时必填, 前端用此识别是否要清零计时器。",
    )
    stage_label: str | None = Field(
        default=None,
        description="人类可读的阶段名, 如 'Fetching from arXiv'。",
    )
    overall_progress: float = Field(
        default=0.0,
        description="0.0-1.0, 整个 task 的总进度估计。",
    )
    message: str | None = None
    error: str | None = None
    timestamp_ms: int = Field(description="后端时钟的 epoch ms, 前端可用此校准。")
    # A.1 Identity threading
    turn_id: str | None = Field(
        default=None,
        description="TurnId str for the agent turn that triggered this task event.",
    )
