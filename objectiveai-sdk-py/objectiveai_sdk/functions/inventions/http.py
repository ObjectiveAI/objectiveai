"""HTTP functions for function inventions."""

from __future__ import annotations

from typing import TYPE_CHECKING, Union

if TYPE_CHECKING:
    from objectiveai_sdk.client import ObjectiveAI
    from objectiveai_sdk.functions.inventions.request import FunctionInventionCreateParams
    from objectiveai_sdk.functions.inventions.response.streaming import FunctionInventionChunk
    from objectiveai_sdk.functions.inventions.response.unary import FunctionInvention
    from objectiveai_sdk.stream import Stream


async def create_function_invention(
    client: ObjectiveAI,
    params: FunctionInventionCreateParams,
) -> Union[FunctionInvention, Stream[FunctionInventionChunk]]:
    """Create a function invention.

    If ``params.stream`` is true, returns a streaming response.
    """
    if getattr(params, "stream", None):
        return await client.post_streaming("functions/inventions", params)
    return await client.post_unary("functions/inventions", params)
