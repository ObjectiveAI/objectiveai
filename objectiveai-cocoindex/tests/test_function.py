"""Function class tests — uses mocked HTTP layer + sentinel clients. No network."""

from __future__ import annotations

from unittest.mock import AsyncMock, patch

import pytest

import objectiveai_cocoindex._client as _client_mod
from objectiveai_cocoindex import (
    Function,
    RemoteFunction,
    RemoteProfile,
    set_default_client,
)
from objectiveai.functions.executions.request.strategy import (
    Strategy,
    StrategyDefault,
    StrategySwissSystem,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


_USAGE_STUB = {
    "prompt_tokens": 0,
    "completion_tokens": 0,
    "total_tokens": 0,
    "cost": 0,
    "total_cost": 0,
}


def _scalar_response(score: float) -> dict:
    return {
        "id": "fn_test",
        "object": "scalar.function.execution",
        "created": 0,
        "tasks": [],
        "tasks_errors": False,
        "usage": _USAGE_STUB,
        "output": {"output": score},
    }


def _make_function(*, strategy=None, client=None) -> Function:
    return Function(
        function=RemoteFunction.mock(name="fn-test"),
        profile=RemoteProfile.mock(name="profile-test"),
        strategy=strategy,
        client=client,
    )


@pytest.fixture(autouse=True)
def _reset_default_client():
    # Each test starts with a clean default-client slot.
    _client_mod._default_instance = None
    yield
    _client_mod._default_instance = None


# ---------------------------------------------------------------------------
# Construction + memo key
# ---------------------------------------------------------------------------


def test_construction_stores_args():
    rf = RemoteFunction.mock(name="x")
    rp = RemoteProfile.mock(name="y")
    strat = Strategy(root=StrategyDefault.model_validate({"type": "default"}))
    sentinel = object()
    fn = Function(rf, rp, strat, client=sentinel)
    assert fn._function is rf
    assert fn._profile is rp
    assert fn._strategy is strat
    assert fn._client is sentinel


def test_memo_key_excludes_client():
    rf = RemoteFunction.github(owner="a", repository="b", commit="c")
    rp = RemoteProfile.github(owner="a", repository="b", commit="c")
    a = Function(rf, rp, client=object())
    b = Function(rf, rp, client=object())
    assert a.__coco_memo_key__() == b.__coco_memo_key__()


def test_memo_key_changes_with_function():
    rp = RemoteProfile.github(owner="a", repository="b", commit="c")
    a = Function(RemoteFunction.github(owner="a", repository="b", commit="c"), rp)
    b = Function(RemoteFunction.github(owner="a", repository="b", commit="DIFF"), rp)
    assert a.__coco_memo_key__() != b.__coco_memo_key__()


def test_memo_key_changes_with_profile():
    rf = RemoteFunction.github(owner="a", repository="b", commit="c")
    a = Function(rf, RemoteProfile.github(owner="a", repository="b", commit="c"))
    b = Function(rf, RemoteProfile.github(owner="a", repository="b", commit="DIFF"))
    assert a.__coco_memo_key__() != b.__coco_memo_key__()


def test_memo_key_changes_with_strategy():
    rf = RemoteFunction.mock(name="x")
    rp = RemoteProfile.mock(name="y")
    a = Function(rf, rp, strategy=None)
    b = Function(rf, rp, strategy=Strategy(root=StrategyDefault.model_validate({"type": "default"})))
    c = Function(
        rf, rp,
        strategy=Strategy(root=StrategySwissSystem.model_validate({
            "type": "swiss_system", "pool": 8, "rounds": 3,
        })),
    )
    assert a.__coco_memo_key__() != b.__coco_memo_key__()
    assert a.__coco_memo_key__() != c.__coco_memo_key__()
    assert b.__coco_memo_key__() != c.__coco_memo_key__()


# ---------------------------------------------------------------------------
# Call mechanics
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_call_builds_correct_request_params():
    fn = _make_function()
    captured = []

    async def fake_execute(client, params):
        captured.append((client, params))
        return _scalar_response(0.42)

    with patch(
        "objectiveai_cocoindex._function.create_function_execution",
        side_effect=fake_execute,
    ):
        await fn({"hello": "world"})

    assert len(captured) == 1
    _, params = captured[0]
    assert params.function.root.root.root.remote == "mock"
    assert params.function.root.root.root.name == "fn-test"
    assert params.profile.root.root.root.remote == "mock"
    assert params.profile.root.root.root.name == "profile-test"
    assert params.strategy is None  # default
    # `input` is required and gets pydantic-validated into InputValue.
    # The dict {"hello": "world"} round-trips through InputValueObject.
    assert params.input is not None


@pytest.mark.asyncio
async def test_call_returns_parsed_function_execution():
    from objectiveai.functions.executions.response.unary import FunctionExecution
    fn = _make_function()
    with patch(
        "objectiveai_cocoindex._function.create_function_execution",
        new_callable=AsyncMock,
        return_value=_scalar_response(0.5),
    ):
        result = await fn({"x": 1})
    assert isinstance(result, FunctionExecution)
    assert result.output.output.root.root == pytest.approx(0.5)


# ---------------------------------------------------------------------------
# Client resolution
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_call_uses_explicit_client():
    sentinel = object()
    fn = _make_function(client=sentinel)
    captured_client = []

    async def fake_execute(client, params):
        captured_client.append(client)
        return _scalar_response(0.0)

    with patch(
        "objectiveai_cocoindex._function.create_function_execution",
        side_effect=fake_execute,
    ):
        await fn({})

    assert captured_client[0] is sentinel


@pytest.mark.asyncio
async def test_call_uses_default_client_when_omitted():
    sentinel = object()
    set_default_client(sentinel)
    fn = _make_function(client=None)
    captured_client = []

    async def fake_execute(client, params):
        captured_client.append(client)
        return _scalar_response(0.0)

    with patch(
        "objectiveai_cocoindex._function.create_function_execution",
        side_effect=fake_execute,
    ):
        await fn({})

    assert captured_client[0] is sentinel


@pytest.mark.asyncio
async def test_call_lazy_constructs_default_client_from_env():
    set_default_client(None)  # ensure unset
    fn = _make_function(client=None)
    constructed = []

    class FakeClient:
        def __init__(self, *args, **kwargs):
            constructed.append((args, kwargs))

    async def fake_execute(client, params):
        return _scalar_response(0.0)

    with patch("objectiveai_cocoindex._client.ObjectiveAI", FakeClient), patch(
        "objectiveai_cocoindex._function.create_function_execution",
        side_effect=fake_execute,
    ):
        await fn({})

    assert len(constructed) == 1


# ---------------------------------------------------------------------------
# Strategy passthrough
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_strategy_none_passes_through():
    fn = _make_function(strategy=None)
    captured = []

    async def fake_execute(client, params):
        captured.append(params)
        return _scalar_response(0.0)

    with patch(
        "objectiveai_cocoindex._function.create_function_execution",
        side_effect=fake_execute,
    ):
        await fn({})

    assert captured[0].strategy is None


@pytest.mark.asyncio
async def test_strategy_set_passes_through():
    strat = Strategy(root=StrategySwissSystem.model_validate({
        "type": "swiss_system", "pool": 4, "rounds": 2,
    }))
    fn = _make_function(strategy=strat)
    captured = []

    async def fake_execute(client, params):
        captured.append(params)
        return _scalar_response(0.0)

    with patch(
        "objectiveai_cocoindex._function.create_function_execution",
        side_effect=fake_execute,
    ):
        await fn({})

    assert captured[0].strategy is not None
    assert isinstance(captured[0].strategy.root, StrategySwissSystem)
    assert captured[0].strategy.root.pool == 4
    assert captured[0].strategy.root.rounds == 2
