"""Single-class wrapper that runs an ObjectiveAI function execution as a
memoized cocoindex processing component.

Bind the (function, profile, strategy) triple at construction; call with
per-execution ``input``. Memo key combines all four. The optional
``client`` is intentionally excluded from the memo key — two ``Function``
instances over the same triple but with different clients share cache
entries.
"""

from __future__ import annotations

from typing import Any

import cocoindex as coco
from objectiveai.client import ObjectiveAI
from objectiveai.functions.executions.http import create_function_execution
from objectiveai.functions.executions.request import FunctionExecutionCreateParams
from objectiveai.functions.executions.request.strategy import Strategy
from objectiveai.functions.executions.response.unary import FunctionExecution

from objectiveai_cocoindex._client import resolve_client
from objectiveai_cocoindex._sources import FunctionSource, ProfileSource


class Function:
    """An ObjectiveAI function bound to a (function, profile, strategy)
    triple, callable as a memoized cocoindex processing component.

    Memo key combines the three constructor args + the per-call ``input``.
    The optional ``client`` is intentionally excluded — two ``Function``
    instances over the same triple with different clients share cache
    entries.

    Example::

        import objectiveai_cocoindex as oai_coco

        scorer = oai_coco.Function(
            function=oai_coco.RemoteFunction.github(
                owner="ObjectiveAI", repository="example-quality", commit="abc"
            ),
            profile=oai_coco.RemoteProfile.github(
                owner="ObjectiveAI", repository="example-quality", commit="abc"
            ),
        )

        execution = await scorer({"text": "hello"})
        out = execution.output.output.root
        # out is one of TaskOutputScalar | TaskOutputVector |
        #               TaskOutputVectors | TaskOutputErr
    """

    __slots__ = ("_function", "_profile", "_strategy", "_client")

    def __init__(
        self,
        function: FunctionSource,
        profile: ProfileSource,
        strategy: Strategy | None = None,
        *,
        client: ObjectiveAI | None = None,
    ) -> None:
        self._function = function
        self._profile = profile
        self._strategy = strategy
        self._client = client

    def __coco_memo_key__(self) -> object:
        # Excludes self._client by design — clients should not invalidate
        # cache entries.
        return (
            "objectiveai_cocoindex.Function",
            self._function.__coco_memo_key__(),
            self._profile.__coco_memo_key__(),
            self._strategy.model_dump() if self._strategy is not None else None,
        )

    @coco.fn(memo=True, logic_tracking="self")
    async def __call__(self, input: Any) -> FunctionExecution:
        client = self._client if self._client is not None else resolve_client()
        params = FunctionExecutionCreateParams(
            function=self._function.to_function_field(),
            profile=self._profile.to_profile_field(),
            strategy=self._strategy,
            input=input,
        )
        result = await create_function_execution(client, params)
        if isinstance(result, FunctionExecution):
            return result
        return FunctionExecution.model_validate(result)
