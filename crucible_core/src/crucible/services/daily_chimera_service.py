"""Daily Chimera pipeline: fetch → ingest → triage → Telegram."""

from __future__ import annotations

import asyncio
import html
import logging
from pathlib import Path
from typing import Any
from urllib.parse import quote

from src.crucible.core.config import ChimeraConfig, get_config, PaperMinerSettings
from src.crucible.core.naming import sanitize_filename
from src.crucible.core.schemas import BatchFilterStats
from src.oligo.core.schemas import Artifact, ToolOutput
from src.crucible.ports.notify.telegram_notifier import TelegramNotifier
from src.crucible.services.batch_filter_workflow import filter_queue_worker
from src.crucible.ports.arxiv.arxiv_fetch import ArxivFetcher
from src.crucible.ports.ingest.mineru_pipeline import convert_queue_worker
from src.crucible.services.task_service import TaskService

logger = logging.getLogger(__name__)


class DailyPipelineStage:
    ARXIV_FETCH = ("arxiv_fetch", "Fetching from arXiv")
    PDF_INGESTION = ("pdf_ingestion", "Converting PDF → Markdown via MinerU")
    BATCH_FILTER = ("batch_filter", "Filtering papers via LLM judge")
    TELEGRAM_NOTIFY = ("telegram_notify", "Sending Telegram digest")


def _merge_arxiv_overrides(
    settings: ChimeraConfig,
    arxiv_query: str | None,
    arxiv_max_results: int | None,
) -> ChimeraConfig:
    """Return config with paper_miner arxiv_query / arxiv_max_results overridden when set."""
    if arxiv_query is None and arxiv_max_results is None:
        return settings
    pm: PaperMinerSettings = settings.paper_miner_or_default
    updates: dict[str, str | int] = {}
    if arxiv_query is not None:
        q = str(arxiv_query).strip()
        if q:
            updates["arxiv_query"] = q
    if arxiv_max_results is not None:
        try:
            n = int(arxiv_max_results)
            updates["arxiv_max_results"] = max(1, min(n, 2000))
        except (TypeError, ValueError):
            pass
    if not updates:
        return settings
    new_pm = pm.model_copy(update=updates)
    return settings.model_copy(update={"paper_miner": new_pm})


def _update_task_progress(
    task_service: TaskService | None,
    task_id: str | None,
    progress: float,
    message: str,
) -> None:
    if task_service is not None and task_id is not None:
        task_service.update_progress(task_id, progress, message)


def _collect_must_read_lines(stats: BatchFilterStats) -> list[str]:
    lines = []
    for item in stats.must_read_items:
        paper_id = str(item.id).strip()
        short_moniker = str(item.short_moniker).strip()
        legacy_title = str(item.title).strip()
        if short_moniker:
            title = f"{paper_id} {short_moniker}".strip() if paper_id else short_moniker
        elif legacy_title:
            title = legacy_title
        else:
            title = paper_id
        lines.append(f"  {title} [{item.score}/10]")
    return lines


def _collect_pipeline_artifacts(
    stats: BatchFilterStats, inbox_folder: Path
) -> list[Artifact]:
    artifacts = []
    for item in stats.must_read_items:
        moniker = sanitize_filename(item.short_moniker) if item.short_moniker else ""
        basename = f"{item.id}-{moniker}" if moniker else sanitize_filename(item.id)
        path = str(inbox_folder / "Must_Read" / f"{basename}.md")
        artifacts.append(
            Artifact(
                kind="vault_note",
                path=path,
                metadata={"arxiv_id": item.id, "verdict": "must_read", "score": item.score},
            )
        )
    return artifacts


def run_daily_pipeline(
    settings: ChimeraConfig | None = None,
    *,
    arxiv_query: str | None = None,
    arxiv_max_results: int | None = None,
    skip_telegram: bool = False,
    task_id: str | None = None,
    task_service: TaskService | None = None,
) -> str:
    """Full Chimera daily path with producer-consumer pipelining.

    Three concurrent stages joined by bounded queues:
      download (semaphore=3) → pdf_queue → convert (single GPU worker) → md_queue → filter (semaphore=3)

    Returns a short text summary.
    """
    summary, _stats = asyncio.run(
        _run_pipelined_async(
            settings=settings,
            arxiv_query=arxiv_query,
            arxiv_max_results=arxiv_max_results,
            skip_telegram=skip_telegram,
            task_id=task_id,
            task_service=task_service,
        )
    )
    return summary


async def run_daily_pipeline_with_stage_events(
    *,
    task_id: str,
    task_service: TaskService,
    settings: ChimeraConfig | None = None,
    arxiv_query: str | None = None,
    arxiv_max_results: int | None = None,
    skip_telegram: bool = False,
) -> str:
    """Async path for task bus + SSE consumers. Delegates to the same pipelined core."""
    if settings is None:
        settings = get_config()
    summary, stats = await _run_pipelined_async(
        settings=settings,
        arxiv_query=arxiv_query,
        arxiv_max_results=arxiv_max_results,
        skip_telegram=skip_telegram,
        task_id=task_id,
        task_service=task_service,
    )
    inbox_folder = settings.require_path("inbox_folder")
    artifacts = _collect_pipeline_artifacts(stats, inbox_folder)
    return ToolOutput(text=summary, artifacts=artifacts if artifacts else None).model_dump_json()


async def _run_pipelined_async(
    settings: ChimeraConfig | None,
    *,
    arxiv_query: str | None,
    arxiv_max_results: int | None,
    skip_telegram: bool,
    task_id: str | None,
    task_service: TaskService | None,
) -> tuple[str, BatchFilterStats]:
    """Core pipeline: download → convert → filter, three concurrent stages via asyncio.Queue.

    Returns (summary_str, stats) so both the sync wrapper and the stage-events wrapper
    can build their own return value from the same data.
    """
    if settings is None:
        settings = get_config()
    settings = _merge_arxiv_overrides(settings, arxiv_query, arxiv_max_results)
    settings.ensure_directories()

    logger.info("[Service] === Chimera Daily Pipeline Started (pipelined) ===")
    pm = settings.paper_miner_or_default

    if task_service is not None and task_id is not None:
        await task_service.start_stage(
            task_id,
            stage_id=DailyPipelineStage.ARXIV_FETCH[0],
            stage_label=DailyPipelineStage.ARXIV_FETCH[1],
            overall_progress=0.0,
        )

    fetcher = ArxivFetcher(settings=settings)
    paper_records = await asyncio.to_thread(fetcher.fetch_metadata)
    new_pdfs_count_holder: list[int] = [0]
    logger.info("[Service] Arxiv metadata fetched. records=%s", len(paper_records))
    _update_task_progress(
        task_service, task_id, 0.1, f"Metadata fetched ({len(paper_records)} records). Starting pipeline..."
    )

    pdf_queue: asyncio.Queue[Path | None] = asyncio.Queue(maxsize=5)
    md_queue: asyncio.Queue[Path | None] = asyncio.Queue(maxsize=5)
    download_sem = asyncio.Semaphore(3)
    stats = BatchFilterStats()
    stats_lock = asyncio.Lock()

    normalized_raw = pm.md_papers_raw_dir.resolve()
    normalized_clean = pm.md_papers_dir.resolve()

    async def _download_stage() -> None:
        count = await fetcher.download_pdfs_to_queue(
            paper_records=paper_records,
            target_dir=pm.arxivpdf_dir,
            pdf_queue=pdf_queue,
            semaphore=download_sem,
        )
        new_pdfs_count_holder[0] = count
        _update_task_progress(
            task_service, task_id, 0.2, f"Downloads done ({count} new PDFs). Converting..."
        )

    if task_service is not None and task_id is not None:
        await task_service.start_stage(
            task_id,
            stage_id=DailyPipelineStage.PDF_INGESTION[0],
            stage_label=DailyPipelineStage.PDF_INGESTION[1],
            overall_progress=0.2,
        )

    filter_workers = [
        asyncio.create_task(
            filter_queue_worker(md_queue, stats, stats_lock, settings=settings),
            name=f"filter-worker-{i}",
        )
        for i in range(3)
    ]

    convert_task = asyncio.create_task(
        convert_queue_worker(pdf_queue, md_queue, normalized_raw, normalized_clean),
        name="convert-worker",
    )
    download_task = asyncio.create_task(_download_stage(), name="download-stage")

    await asyncio.gather(download_task, convert_task, *filter_workers)

    ingested_count = convert_task.result()
    new_pdfs_count = new_pdfs_count_holder[0]

    logger.info(
        "[Service] Pipeline stages done. new_pdfs=%s ingested=%s filtered=%s",
        new_pdfs_count, ingested_count, stats.total,
    )

    if task_service is not None and task_id is not None:
        await task_service.start_stage(
            task_id,
            stage_id=DailyPipelineStage.BATCH_FILTER[0],
            stage_label=DailyPipelineStage.BATCH_FILTER[1],
            overall_progress=0.6,
        )
    _update_task_progress(
        task_service,
        task_id,
        0.95,
        f"Triage done: total={stats.total} must_read={stats.must_read} "
        f"skim={stats.skim} reject={stats.reject} errors={stats.errors}.",
    )

    if not skip_telegram:
        if task_service is not None and task_id is not None:
            await task_service.start_stage(
                task_id,
                stage_id=DailyPipelineStage.TELEGRAM_NOTIFY[0],
                stage_label=DailyPipelineStage.TELEGRAM_NOTIFY[1],
                overall_progress=0.95,
            )
        report_message, reply_markup = _render_daily_report(stats=stats, new_pdfs_count=new_pdfs_count)
        notifier = TelegramNotifier(settings=settings)
        await asyncio.to_thread(
            notifier.send_summary, html_message=report_message, reply_markup=reply_markup
        )
        _update_task_progress(task_service, task_id, 0.99, "Telegram sent.")
    else:
        _update_task_progress(task_service, task_id, 0.99, "Telegram skipped.")

    summary = (
        f"Daily pipeline completed. new_pdfs={new_pdfs_count} ingested={ingested_count} "
        f"batch_total={stats.total} must_read={stats.must_read} skim={stats.skim} "
        f"reject={stats.reject} errors={stats.errors} telegram={'no' if skip_telegram else 'yes'}"
    )
    must_read_lines = _collect_must_read_lines(stats)
    if must_read_lines:
        summary += "\nMust Read:\n" + "\n".join(must_read_lines)
    logger.info("[Service] %s", summary)
    return summary, stats


def _render_daily_report(
    stats: BatchFilterStats,
    new_pdfs_count: int,
) -> tuple[str, dict[str, list[list[dict[str, str]]]] | None]:
    total = int(stats.total)
    must_read = int(stats.must_read)
    skim = int(stats.skim)
    reject = int(stats.reject)

    items_raw = stats.must_read_items
    must_read_items: list[dict[str, Any]] = []
    inline_keyboard: list[list[dict[str, str]]] = []
    for item in items_raw:
        score = item.score
        paper_id = str(item.id).strip()
        filename = str(item.filename).strip()
        short_moniker = str(item.short_moniker).strip()
        legacy_title = str(item.title).strip()
        if short_moniker:
            title = f"{paper_id} {short_moniker}".strip() if paper_id else short_moniker
        elif legacy_title:
            title = legacy_title
        else:
            title = paper_id
        novelty = item.novelty
        encoded_id = quote(paper_id, safe="")
        arxiv_url = f"https://arxiv.org/abs/{encoded_id}" if paper_id else "#"
        _router_base = get_config().system.vault_router_url or "https://chimeravaultrouter.haydenshui.workers.dev"
        obsidian_url = (
            f"{_router_base}/?id={encoded_id}"
            if paper_id
            else "#"
        )
        short_for_button_paper = f"Paper {encoded_id}"
        short_for_button_obsidian = (
            f"Node for {short_moniker}" if short_moniker else f"Node for {encoded_id}"
        )
        inline_keyboard.append(
            [
                {"text": f"🌐 {short_for_button_paper}", "url": arxiv_url},
                {"text": f"🧠 {short_for_button_obsidian}", "url": obsidian_url},
            ]
        )
        must_read_items.append(
            {
                "score": int(score),
                "id": paper_id,
                "filename": filename,
                "title": html.escape(str(title), quote=False),
                "novelty": html.escape(str(novelty), quote=False),
            }
        )

    if not must_read_items:
        for title in stats.must_read_titles:
            must_read_items.append(
                {
                    "score": 0,
                    "id": "",
                    "filename": "",
                    "title": html.escape(str(title), quote=False),
                    "novelty": "N/A",
                }
            )

    lines: list[str] = [
        "🚨 <b>[BB Channel] Chimera Morning Broadcast</b> 🚨",
        "━━━━━━━━━━━━━━━━━━━━",
        '"Good morning, Senpai~ ♡ Here is the academic trash I\'ve digested for you."',
        "",
        f"📥 New PDFs fetched: <b>{int(new_pdfs_count)}</b>",
        f"📄 Ingested papers: <b>{total}</b>",
        f"💎 Must Read: <b>{must_read}</b>",
        f"🪶 Skim: <b>{skim}</b>",
        f"🗑️ Reject: {reject}",
        "",
        "━━━━━━━━━━━━━━━━━━━━",
        "🎯 <b>SURVIVING TARGETS (Please consume)</b>",
        "",
    ]
    if must_read_items:
        for item in must_read_items:
            lines.append(
                f"🔹 <b>[{item['score']}/10]</b> <code>{item['title']}</code>"
            )
            lines.append(f"   <i>💡 {item['novelty']}</i>")
    else:
        lines.append("<i>☕ All targets were garbage today. You can go back to sleep.</i>")

    html_message = "\n".join(lines).strip()
    reply_markup = {"inline_keyboard": inline_keyboard} if inline_keyboard else None
    return html_message, reply_markup
