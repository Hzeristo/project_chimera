"""Post-triage archival: filtered MD, audit log, source PDF, MinerU raw cleanup."""

from __future__ import annotations

import csv
import logging
import shutil
from datetime import datetime, timezone
from pathlib import Path

from src.crucible.core.config import ChimeraConfig
from src.crucible.core.naming import compute_fancy_basename
from src.crucible.core.schemas import Paper, PaperAnalysisResult

logger = logging.getLogger(__name__)


class PaperArchiveAdapter:
    """Route a triaged paper into ``papers/filtered/{verdict}/`` and append the audit log."""

    def __init__(self, settings: ChimeraConfig) -> None:
        self._settings = settings
        pm = settings.paper_miner_or_default
        if pm.filtered_dir is None or pm.failed_dir is None or pm.papers_root is None:
            raise ValueError(
                "[Archive] paper_miner.filtered_dir / failed_dir / papers_root "
                "must resolve to absolute paths."
            )
        self._filtered_dir: Path = pm.filtered_dir
        self._failed_dir: Path = pm.failed_dir
        self._papers_root: Path = pm.papers_root
        self._arxivpdf_dir: Path | None = pm.arxivpdf_dir

    def route_and_cleanup(
        self, paper: Paper, result: PaperAnalysisResult
    ) -> Path:
        """Move clean MD to ``filtered/{verdict}/``, stage PDF, append audit log."""
        verdict_dir = self._filtered_dir / result.verdict.value.replace(" ", "_")
        verdict_dir.mkdir(parents=True, exist_ok=True)

        fancy_stem = compute_fancy_basename(paper, result)
        target_md = verdict_dir / f"{fancy_stem}.md"
        source_md = paper.content_path

        if not source_md.exists():
            raise RuntimeError(
                f"[Archive] Source clean MD not found, cannot route: {source_md}"
            )

        try:
            if target_md.exists():
                logger.info(
                    "[Archive] Filtered MD already exists, overwriting: %s", target_md
                )
                target_md.unlink()
            shutil.move(str(source_md), str(target_md))
            logger.info("[Archive] Routed MD: %s -> %s", source_md.name, target_md)
        except OSError as exc:
            logger.error(
                "[Archive] Failed moving MD %s -> %s",
                source_md,
                target_md,
                exc_info=True,
            )
            raise RuntimeError(f"Failed routing MD to {target_md}") from exc

        if self._arxivpdf_dir is not None and self._arxivpdf_dir.is_dir():
            pdf_src = self._arxivpdf_dir / f"{paper.id}.pdf"
            if pdf_src.exists():
                pdf_dst = verdict_dir / pdf_src.name
                try:
                    if pdf_dst.exists():
                        pdf_dst.unlink()
                    shutil.move(str(pdf_src), str(pdf_dst))
                    logger.info(
                        "[Archive] Routed PDF: %s -> %s", pdf_src.name, pdf_dst
                    )
                except OSError as exc:
                    logger.warning(
                        "[Archive] Failed routing PDF %s -> %s: %s",
                        pdf_src,
                        pdf_dst,
                        exc,
                    )

        audit_path = self._papers_root / "audit_log.csv"
        try:
            audit_path.parent.mkdir(parents=True, exist_ok=True)
            write_header = not audit_path.exists()
            with audit_path.open("a", encoding="utf-8", newline="") as fh:
                writer = csv.writer(fh)
                if write_header:
                    writer.writerow(
                        (
                            "timestamp",
                            "paper_id",
                            "verdict",
                            "score",
                            "short_moniker",
                            "filtered_md",
                        )
                    )
                writer.writerow(
                    (
                        datetime.now(timezone.utc).isoformat(timespec="seconds"),
                        paper.id,
                        result.verdict.value,
                        int(result.score),
                        result.short_moniker,
                        str(target_md),
                    )
                )
        except OSError as exc:
            logger.error(
                "[Archive] Failed appending audit log %s", audit_path, exc_info=True
            )
            raise RuntimeError(f"Audit log write failed: {audit_path}") from exc

        return target_md

    def route_failed_cleanup(
        self, *, paper_id: str, md_path: Path
    ) -> Path | None:
        """Move a failed-to-process MD into ``failed/`` for later inspection."""
        if not md_path.exists():
            logger.info(
                "[Archive] Failed cleanup skipped, source missing: %s", md_path
            )
            return None
        self._failed_dir.mkdir(parents=True, exist_ok=True)
        target = self._failed_dir / f"{paper_id}.md"
        try:
            if target.exists():
                target.unlink()
            shutil.move(str(md_path), str(target))
            logger.info(
                "[Archive] Failed MD moved aside: %s -> %s", md_path.name, target
            )
        except OSError as exc:
            logger.error(
                "[Archive] Failed moving %s to failed dir", md_path, exc_info=True
            )
            raise RuntimeError(f"Failed cleanup move to {target}") from exc
        return target

    def cleanup_playground(
        self, raw_dir: Path, clean_md: Path | None
    ) -> None:
        """Remove MinerU staging artifacts; only delete clean MD if still under md_papers_dir."""
        if raw_dir.exists() and raw_dir.is_dir():
            try:
                shutil.rmtree(raw_dir)
                logger.info("[Archive] Cleaned MinerU raw dir: %s", raw_dir)
            except OSError as exc:
                logger.warning(
                    "[Archive] Failed to remove raw dir %s: %s", raw_dir, exc
                )

        if clean_md is None:
            return
        pm = self._settings.paper_miner_or_default
        if pm.md_papers_dir is None:
            return
        try:
            clean_md.resolve().relative_to(pm.md_papers_dir.resolve())
        except ValueError:
            return
        if clean_md.exists():
            try:
                clean_md.unlink()
                logger.info("[Archive] Cleaned leftover clean MD: %s", clean_md)
            except OSError as exc:
                logger.warning(
                    "[Archive] Failed to remove leftover clean MD %s: %s",
                    clean_md,
                    exc,
                )
