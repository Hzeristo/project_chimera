"""Staging protocol HTTP endpoints — UI→HTTP→StagingService (no LLM involvement)."""

from __future__ import annotations

from pathlib import Path

from fastapi import APIRouter, HTTPException, Request
from pydantic import BaseModel

from src.crucible.services.staging_service import StagingService

router = APIRouter(prefix="/v1/staging")


def _svc(request: Request) -> StagingService:
    settings = request.app.state.settings
    return StagingService(
        staging_dir=settings.system.staging_dir,
        vault_root=settings.require_path("vault_root"),
    )


class CreateRequest(BaseModel):
    type: str
    title: str
    body: str
    edges: dict | None = None


class PathRequest(BaseModel):
    staging_path: str


@router.post("/create")
async def create_staging(body: CreateRequest, request: Request) -> dict:
    try:
        path = _svc(request).create_staging_node(body.type, body.title, body.body, body.edges)
        return {"path": str(path)}
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))


@router.get("/list")
async def list_staging(request: Request) -> dict:
    staging_dir = request.app.state.settings.system.staging_dir
    if not staging_dir.exists():
        return {"candidates": []}
    return {"candidates": [str(p) for p in sorted(staging_dir.glob("*.md"))]}


@router.post("/promote")
async def promote_staging(body: PathRequest, request: Request) -> dict:
    vault_path = _svc(request).promote_node(Path(body.staging_path))
    return {"vault_path": str(vault_path)}


@router.post("/reject")
async def reject_staging(body: PathRequest, request: Request) -> dict:
    _svc(request).reject_node(Path(body.staging_path))
    return {}
