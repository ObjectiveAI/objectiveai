"""Shared utilities for HTTP integration tests.

Requires a running ObjectiveAI API server. Set OBJECTIVEAI_TEST_PORT
environment variable to the server's port.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any, Callable, Type

import pytest

from tests.push_test_utils import rounded

ASSETS_DIR = Path(__file__).resolve().parent.parent.parent / "objectiveai-api" / "assets"

_port = os.environ.get("OBJECTIVEAI_TEST_PORT")


class _AttrDict(dict):
    """Dict subclass that exposes keys as attributes.

    HTTP functions use ``getattr(params, "stream", None)`` to route,
    while ``json.dumps`` needs a dict.  This satisfies both.
    """

    def __getattr__(self, name: str) -> Any:
        try:
            return self[name]
        except KeyError:
            raise AttributeError(name)


def get_test_client():
    """Create a test client connected to the local test server."""
    if not _port:
        pytest.skip("OBJECTIVEAI_TEST_PORT not set")
    from objectiveai_sdk.client import ObjectiveAI
    return ObjectiveAI(address=f"http://127.0.0.1:{_port}")


def load_snapshot(snapshots_dir: Path, name: str) -> dict:
    """Load a snapshot JSON file."""
    return json.loads((snapshots_dir / f"{name}.json").read_text(encoding="utf-8"))


class HttpTestCase:
    """A single HTTP test case."""

    def __init__(
        self,
        snapshot: str,
        body: dict,
    ):
        self.snapshot = snapshot
        self.body = body


def http_test_suite(
    *,
    name: str,
    fn: Callable,
    snapshots_dir: Path,
    chunk_cls: Type,
    chunk_to_unary: Callable[[dict], dict],
    normalize: Callable[[dict], dict],
    cases: list[HttpTestCase],
):
    """Generate a parametrized HTTP test suite (unary + streaming).

    Mirrors httpTestSuite() from objectiveai-js/src/httpTestUtil.ts.
    ``fn`` is the exported HTTP function from the corresponding http.py
    module (e.g. ``create_agent_completion``).
    Returns a dict of test functions that pytest will collect from the
    caller's module globals.
    """

    async def _post_unary(client, body: dict) -> dict:
        return await fn(client, _AttrDict({**body, "stream": False}))

    async def _post_streaming(client, body: dict) -> dict:
        stream = await fn(client, _AttrDict({**body, "stream": True}))
        acc = None
        async for raw_chunk in stream:
            chunk = chunk_cls.model_validate(raw_chunk)
            if acc is None:
                acc = chunk
            else:
                acc.push(chunk)
        assert acc is not None, "Stream yielded no chunks"
        return chunk_to_unary(acc.model_dump(mode="python", by_alias=True, exclude_unset=True))

    @pytest.fixture
    def client():
        return get_test_client()

    @pytest.mark.asyncio
    @pytest.mark.parametrize("case", cases, ids=[c.snapshot for c in cases])
    async def test_unary(client, case):
        expected = rounded(load_snapshot(snapshots_dir, case.snapshot))
        result = rounded(normalize(await _post_unary(client, case.body)))
        assert result == expected

    @pytest.mark.asyncio
    @pytest.mark.parametrize("case", cases, ids=[c.snapshot for c in cases])
    async def test_streaming(client, case):
        expected = rounded(load_snapshot(snapshots_dir, case.snapshot))
        result = rounded(normalize(await _post_streaming(client, case.body)))
        assert result == expected

    return {"client": client, "test_unary": test_unary, "test_streaming": test_streaming}
