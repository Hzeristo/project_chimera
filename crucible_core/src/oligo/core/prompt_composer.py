"""统一 system / message prompt 组装器；默认片段见 ``_register_default_components``。"""

from __future__ import annotations

import json
import logging
import re
from collections.abc import Collection
from pathlib import Path
from typing import Any
from xml.etree import ElementTree as ET

from jinja2 import Environment, FileSystemLoader, StrictUndefined

from src.crucible.core.schemas import PromptComponent, PromptStage, ToolSpec
from src.oligo.tools.registry import get_tool_registry

_global_composer: PromptComposer | None = None
logger = logging.getLogger(__name__)

_PROMPTS_DIR = Path(__file__).resolve().parent.parent / "prompts"
_jinja_env = Environment(
    loader=FileSystemLoader(str(_PROMPTS_DIR)),
    trim_blocks=True,
    lstrip_blocks=True,
    undefined=StrictUndefined,
    keep_trailing_newline=False,
)


def _load_j2(name: str) -> str:
    """Render a Jinja2 template at registration time (no variables).

    Result is escaped for str.format() so literal braces survive compose().
    """
    rendered = _jinja_env.get_template(name).render().rstrip("\n")
    return rendered.replace("{", "{{").replace("}", "}}")


def _load_md(name: str) -> str:
    """Load a plain .md template for compose-time str.format()."""
    return (_PROMPTS_DIR / name).read_text(encoding="utf-8").rstrip("\n")


# Exported for tests that assert on guardrail content.
FINAL_GUARDRAIL_TEXT: str = _load_j2("final_guardrail.md.j2")


def _param_meta(raw: Any) -> dict[str, Any]:
    return raw if isinstance(raw, dict) else {"type": "str", "required": False}


def _placeholder_for_param_type(type_str: str) -> Any:
    t = (type_str or "str").lower()
    if t in ("int", "integer"):
        return 5
    if t in ("bool", "boolean"):
        return False
    if t in ("float", "double", "number"):
        return 0.5
    return "..."


def _example_args_for_spec(spec: ToolSpec) -> dict[str, Any]:
    """Build a minimal JSON object: all required keys with type placeholders; else {{}}."""
    schema = spec.args_schema or {}
    if not schema:
        return {}
    required_keys = [
        k
        for k, v in schema.items()
        if bool(_param_meta(v).get("required"))
    ]
    if not required_keys:
        return {}
    out: dict[str, Any] = {}
    for k in schema:
        if k not in required_keys:
            continue
        meta = _param_meta(schema[k])
        out[k] = _placeholder_for_param_type(str(meta.get("type", "str")))
    return out


def _format_one_tool_verbose(spec: ToolSpec) -> str:
    schema = spec.args_schema or {}
    title = spec.name
    if spec.long_running:
        title += " [long_running → returns task_id; poll with the status tool from the same list]"
    lines = [f"### {title}", spec.description.strip()]
    if not schema:
        lines.append("Parameters: (none declared; use {{}} if the tool accepts only optional keys)")
    else:
        lines.append("Parameters:")
        for pname, raw in schema.items():
            meta = _param_meta(raw)
            typ = str(meta.get("type", "str"))
            req_lbl = "required" if bool(meta.get("required")) else "optional"
            help_t = str(meta.get("help", "")).strip()
            if help_t:
                lines.append(f"  - {pname} ({typ}, {req_lbl}): {help_t}")
            else:
                lines.append(f"  - {pname} ({typ}, {req_lbl})")
    ex = _example_args_for_spec(spec)
    payload = json.dumps(ex, ensure_ascii=False)
    lines.append(
        f"Example (XML): <tool_call name=\"{spec.name}\"><args>{payload}</args></tool_call>"
    )
    lines.append(f"Example (CMD): <CMD:{spec.name}({payload})>")
    return "\n".join(lines)


def _format_one_tool_compact(spec: ToolSpec) -> str:
    ex = _example_args_for_spec(spec)
    payload = json.dumps(ex, ensure_ascii=False)
    lr = " [long_running]" if spec.long_running else ""
    desc = (spec.description or "").strip()
    if len(desc) > 160:
        desc = desc[:157] + "…"
    return f"- {spec.name}{lr}: {desc} | ex: <CMD:{spec.name}({payload})>"


def _format_one_tool_micro(spec: ToolSpec) -> str:
    lr = " [long_running]" if spec.long_running else ""
    return f"- {spec.name}{lr}"


def _render_tool_list(
    allowed: Collection[str] | None = None,
    max_chars: int | None = None,
) -> str:
    """从 ToolRegistry 生成 router 工具块（与 ``router_tool_registry`` 的 ``{{tool_list}}`` 对应）。"""
    specs = get_tool_registry().list_specs(allowed=allowed)
    if not specs:
        msg = "- (no tools are available in this session)"
        if max_chars is not None and len(msg) > max_chars:
            return msg[: max(0, max_chars)]
        return msg

    verbose = "\n\n".join(_format_one_tool_verbose(s) for s in specs)
    if max_chars is None or len(verbose) <= max_chars:
        return verbose

    compact = "\n".join(_format_one_tool_compact(s) for s in specs)
    if len(compact) <= max_chars:
        logger.warning(
            "[Prompt] tool_list downgraded to compact format (limit=%s)", max_chars
        )
        return compact

    micro = "\n".join(_format_one_tool_micro(s) for s in specs)
    if len(micro) <= max_chars:
        logger.warning(
            "[Prompt] tool_list downgraded to micro format (limit=%s)", max_chars
        )
        return micro

    suffix = "\n…[tool_list truncated]"
    room = max_chars - len(suffix)
    if room < 1:
        logger.warning("[Prompt] tool_list hard truncated (limit=%s)", max_chars)
        return suffix[:max_chars]
    truncated = micro[:room] + suffix
    logger.warning("[Prompt] tool_list hard truncated (limit=%s)", max_chars)
    return truncated if len(truncated) <= max_chars else truncated[:max_chars]


def _sanitize_xml_tag(name: str) -> str:
    """将 dict 键转为合法 XML 标签名（保守替换；vault 场景通常已是简单标识）。"""
    raw = str(name).strip()
    if not raw:
        return "key"
    if raw[0].isdigit():
        raw = "_" + raw
    raw = re.sub(r"[^\w.\-]", "_", raw)
    return raw or "key"


def _xml_scalar_text(value: Any) -> str | None:
    if value is None:
        return None
    if isinstance(value, bool):
        return "true" if value else "false"
    return str(value)


def _xml_append_value(parent: ET.Element, key: str, value: Any) -> None:
    tag = _sanitize_xml_tag(key)
    if isinstance(value, dict):
        el = ET.SubElement(parent, tag)
        for k, v in value.items():
            _xml_append_value(el, k, v)
    elif isinstance(value, list):
        el = ET.SubElement(parent, tag)
        for item in value:
            if isinstance(item, dict):
                item_el = ET.SubElement(el, "item")
                for k, v in item.items():
                    _xml_append_value(item_el, k, v)
            elif isinstance(item, list):
                _xml_append_value(el, "item", item)
            else:
                item_el = ET.SubElement(el, "item")
                t = _xml_scalar_text(item)
                if t is not None:
                    item_el.text = t
    else:
        el = ET.SubElement(parent, tag)
        t = _xml_scalar_text(value)
        if t is not None:
            el.text = t


def _render_xml_structured(data: dict[str, Any]) -> str:
    """将嵌套 dict / list / 标量转为带缩进的 XML 字符串（ElementTree；仅用于 prompt 注入）。"""
    root = ET.Element("structured")
    for k, v in data.items():
        _xml_append_value(root, k, v)
    ET.indent(root, space="  ")
    return ET.tostring(root, encoding="unicode")


class PromptComposer:
    """统一的 system prompt 组装器"""

    def __init__(self) -> None:
        self._components: dict[str, PromptComponent] = {}

    def register(self, component: PromptComponent) -> None:
        """注册一个 prompt 片段"""
        if component.id in self._components:
            raise ValueError(f"Duplicate component id: {component.id}")
        self._components[component.id] = component

    def compose(
        self,
        stage: PromptStage,
        context: dict[str, Any],
        active_ids: set[str] | None = None,
    ) -> tuple[str, str]:
        """
        组装指定 stage 的 system prompt.

        Returns:
            (stable_section, dynamic_section)
        """
        candidates: list[PromptComponent] = []
        for c in self._components.values():
            if _component_matches_stage(c.stage, stage):
                candidates.append(c)

        if active_ids is not None:
            candidates = [c for c in candidates if c.id in active_ids]

        if stage == PromptStage.ROUTER:
            # HOTFIX.3: persona 只允许进入 Final 阶段，Router 侧做二次保险。
            safe_candidates: list[PromptComponent] = []
            for c in candidates:
                if isinstance(c.template, str) and "{persona}" in c.template:
                    logger.warning(
                        "[Prompt] drop persona-bearing component in router stage: %s",
                        c.id,
                    )
                    continue
                safe_candidates.append(c)
            candidates = safe_candidates

        candidates.sort(key=lambda x: x.priority, reverse=True)

        stable_parts: list[str] = []
        dynamic_parts: list[str] = []
        for c in candidates:
            if c.renderer == "xml_structured":
                if not isinstance(c.template, dict):
                    raise TypeError(
                        f"component {c.id!r}: xml_structured template must be dict"
                    )
                rendered = _render_xml_structured(c.template)
            else:
                if not isinstance(c.template, str):
                    raise TypeError(
                        f"component {c.id!r}: text renderer expects str template"
                    )
                rendered = c.template.format(**context)
            if c.cacheable:
                stable_parts.append(rendered)
            else:
                dynamic_parts.append(rendered)

        stable_section = "\n\n".join(stable_parts)
        dynamic_section = "\n\n".join(dynamic_parts)
        return (stable_section, dynamic_section)

    def get_component(self, component_id: str) -> PromptComponent | None:
        return self._components.get(component_id)


def _component_matches_stage(component_stage: PromptStage, target: PromptStage) -> bool:
    if component_stage == PromptStage.BOTH:
        return target in (PromptStage.ROUTER, PromptStage.FINAL)
    if target == PromptStage.BOTH:
        return component_stage == PromptStage.BOTH
    return component_stage == target


def get_prompt_composer() -> PromptComposer:
    global _global_composer
    if _global_composer is None:
        _global_composer = PromptComposer()
        _register_default_components(_global_composer)
    return _global_composer


def _register_default_components(composer: PromptComposer) -> None:
    """注册 ChimeraAgent 与 MW.0 审计对齐的默认片段。"""

    # --- Router ---
    composer.register(
        PromptComponent(
            id="router_core",
            stage=PromptStage.ROUTER,
            priority=100,
            cacheable=True,
            template=_load_j2("router_intro.md.j2"),
        )
    )
    composer.register(
        PromptComponent(
            id="router_tool_registry",
            stage=PromptStage.ROUTER,
            priority=90,
            cacheable=True,
            template="Available tools:\n{tool_list}\n\n" + _load_j2("router_post_tools.md.j2"),
        )
    )
    composer.register(
        PromptComponent(
            id="router_skill_directive",
            stage=PromptStage.ROUTER,
            priority=80,
            cacheable=True,
            template=_load_md("router_skill_directive.md"),
        )
    )
    # --- Final ---
    composer.register(
        PromptComponent(
            id="final_system_core",
            stage=PromptStage.FINAL,
            priority=100,
            cacheable=True,
            template=_load_md("final_system_core.md"),
        )
    )
    composer.register(
        PromptComponent(
            id="final_skill_directive",
            stage=PromptStage.FINAL,
            priority=80,
            cacheable=True,
            template=_load_md("final_skill_directive.md"),
        )
    )
    composer.register(
        PromptComponent(
            id="final_persona_override",
            stage=PromptStage.FINAL,
            priority=60,
            cacheable=True,
            template=_load_md("final_persona_override.md"),
        )
    )
    composer.register(
        PromptComponent(
            id="final_authors_note",
            stage=PromptStage.FINAL,
            priority=40,
            cacheable=True,
            template=_load_md("final_authors_note.md"),
        )
    )
    composer.register(
        PromptComponent(
            id="final_guardrail",
            stage=PromptStage.FINAL,
            priority=10,
            cacheable=True,
            template=FINAL_GUARDRAIL_TEXT,
        )
    )
    composer.register(
        PromptComponent(
            id="dynamic_timestamp",
            stage=PromptStage.BOTH,
            priority=5,
            cacheable=False,
            template="{timestamp}",
        )
    )
