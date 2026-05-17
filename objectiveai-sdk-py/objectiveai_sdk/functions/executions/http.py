"""HTTP functions for function executions."""

from __future__ import annotations

from typing import TYPE_CHECKING, Union

if TYPE_CHECKING:
    from objectiveai_sdk.client import ObjectiveAI
    from objectiveai_sdk.functions.executions.request import FunctionExecutionCreateParams
    from objectiveai_sdk.functions.executions.response.streaming import FunctionExecutionChunk
    from objectiveai_sdk.functions.executions.response.unary import FunctionExecution
    from objectiveai_sdk.stream import Stream


async def create_function_execution(
    client: ObjectiveAI,
    params: FunctionExecutionCreateParams,
) -> Union[FunctionExecution, Stream[FunctionExecutionChunk]]:
    """Execute a function.

    If ``params.stream`` is true, returns a streaming response.
    """
    if getattr(params, "stream", None):
        return await client.post_streaming("functions/executions", params)
    return await client.post_unary("functions/executions", params)
