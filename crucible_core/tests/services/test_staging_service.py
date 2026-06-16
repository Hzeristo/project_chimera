# crucible_core/tests/services/test_staging_service.py
"""Unit tests for StagingService create / promote / reject."""

from __future__ import annotations

import yaml

from src.crucible.services.staging_service import StagingService


def _svc(tmp_path):
    return StagingService(tmp_path / "staging", tmp_path / "vault")


def test_create_staging_node_thought(tmp_path):
    path = _svc(tmp_path).create_staging_node("thought", "My Hypothesis", "Observation.")
    assert path.exists()
    text = path.read_text()
    assert "status: PENDING_REVIEW" in text
    assert "type: thought" in text
    assert "derives_from" in text


def test_create_staging_node_unknown_type(tmp_path):
    import pytest
    with pytest.raises(ValueError):
        _svc(tmp_path).create_staging_node("knowledge", "X", "body")


def test_promote_moves_to_vault_and_sets_active(tmp_path):
    svc = _svc(tmp_path)
    staging_path = svc.create_staging_node("thought", "My Hypothesis", "Body.")
    vault_path = svc.promote_node(staging_path)
    assert vault_path.exists()
    assert not staging_path.exists()
    fm = yaml.safe_load(vault_path.read_text().split("---")[1])
    assert fm["status"] == "active"
    assert vault_path.parent.name == "Thoughts"


def test_promote_insight_destination(tmp_path):
    svc = _svc(tmp_path)
    path = svc.create_staging_node("insight", "Key Insight", "Body.")
    vault_path = svc.promote_node(path)
    assert vault_path.parent.name == "Insight"


def test_reject_node_removes_file(tmp_path):
    svc = _svc(tmp_path)
    path = svc.create_staging_node("decision", "Pick Approach", "Body.")
    svc.reject_node(path)
    assert not path.exists()
