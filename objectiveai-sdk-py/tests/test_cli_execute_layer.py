"""Tests for the cli/command "CLI running" layer.

Covers the hand-written executors + CliStream and the generated per-leaf
``_execute`` functions: the import convention (functions importable from their
leaf package, same as the classes), that every generated module imports and
builds its adapters, and that a generated execute fn injects the path
discriminator + parses the error/response union — all driven by a fake
in-process executor so nothing hits the network or a real binary.
"""
import importlib
import inspect
import pathlib

import pytest

from objectiveai_sdk.cli.command.executor import (
    BinaryCommandExecutor,
    CommandExecutor,
    PluginCommandExecutor,
)


def test_import_convention_streaming():
    # `from <leaf> import execute` resolves to the FUNCTION (not the _execute
    # module) — the same convention as `from <leaf> import Request`.
    from objectiveai_sdk.cli.command.plugins.run import execute, execute_transform

    assert inspect.isfunction(execute)
    # streaming fns return a CliStream synchronously (not a coroutine).
    assert not inspect.iscoroutinefunction(execute)
    assert inspect.isfunction(execute_transform)


def test_import_convention_unary_is_async():
    from objectiveai_sdk.cli.command.agents.get import execute

    assert inspect.iscoroutinefunction(execute)


def test_executors_importable():
    assert BinaryCommandExecutor is not None
    assert PluginCommandExecutor is not None
    assert CommandExecutor is not None


def test_all_execute_modules_import():
    """Every generated `_execute.py` imports cleanly (catches any class-name /
    module-path mismatch and any adapter that fails to build)."""
    root = pathlib.Path(__file__).resolve().parent.parent / "objectiveai_sdk"
    mods = [
        "objectiveai_sdk." + ".".join(p.relative_to(root).with_suffix("").parts)
        for p in root.rglob("_execute.py")
    ]
    assert mods, "expected generated _execute modules"
    for m in mods:
        importlib.import_module(m)


class _FakeExecutor:
    """A CommandExecutor that records the wire request and yields canned,
    already-parsed NDJSON values (what a real executor's stdout would parse to)."""

    def __init__(self, items):
        self._items = items
        self.last_request = None

    async def execute(self, request):
        self.last_request = request
        for item in self._items:
            yield item


@pytest.mark.asyncio
async def test_streaming_execute_injects_path_type_and_parses_error():
    from objectiveai_sdk.cli.command.plugins.run import execute
    from objectiveai_sdk.cli.command.plugins.run.request import Request
    from objectiveai_sdk.cli.error import Error as CliError

    fake = _FakeExecutor([{"type": "error", "message": "boom"}, {"type": "end"}])
    req = Request(owner="o", name="x", version="0.0.1", args=[], path_type="plugins/run")

    stream = execute(fake, req)
    items = await stream.to_list()

    # The end marker is consumed; only the error line surfaces.
    assert len(items) == 1
    assert isinstance(items[0], CliError)  # union is left-to-right: CliError wins
    assert items[0].message == "boom"

    # The discriminator is injected and the transform pair is cleared.
    assert fake.last_request["path_type"] == "plugins/run"
    assert "jq" not in fake.last_request
    assert "python" not in fake.last_request


@pytest.mark.asyncio
async def test_execute_transform_spreads_transform():
    from objectiveai_sdk.cli.command.plugins.run import execute_transform
    from objectiveai_sdk.cli.command.plugins.run.request import Request

    fake = _FakeExecutor([{"type": "end"}])
    req = Request(owner="o", name="x", version="0.0.1", args=[], path_type="plugins/run")

    await execute_transform(fake, req, {"jq": ".foo"}).to_list()

    assert fake.last_request["jq"] == ".foo"
    assert "python" not in fake.last_request
    assert fake.last_request["path_type"] == "plugins/run"


@pytest.mark.asyncio
async def test_cli_stream_unwraps_layers_and_skips_end():
    from pydantic import TypeAdapter

    from objectiveai_sdk.cli.command.cli_stream import CliStream

    async def source():
        yield {"A": {"B": 7}}  # externally-tagged single-key layers, peeled to 7
        yield {"type": "end"}  # terminator, never yielded

    stream = CliStream(source(), TypeAdapter(int))
    assert await stream.to_list() == [7]


@pytest.mark.asyncio
async def test_cli_stream_first_releases_stream():
    from pydantic import TypeAdapter

    from objectiveai_sdk.cli.command.cli_stream import CliStream

    async def source():
        yield 1
        yield 2

    stream = CliStream(source(), TypeAdapter(int))
    assert await stream.first() == 1
