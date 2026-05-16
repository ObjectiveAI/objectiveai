"""HTTP functions for recursive function inventions."""

from __future__ import annotations

from typing import TYPE_CHECKING, Union

if TYPE_CHECKING:
    from objectiveai.client import ObjectiveAI
    from objectiveai.functions.inventions.recursive.request import (
        FunctionInventionRecursiveCreateParams,
    )
    from objectiveai.functions.inventions.recursive.response.streaming import (
        FunctionInventionRecursiveChunk,
    )
    from objectiveai.functions.inventions.recursive.response.unary import (
        FunctionInventionRecursive,
    )
    from objectiveai.stream import Stream


async def create_function_invention_recursive(
    client: ObjectiveAI,
    params: FunctionInventionRecursiveCreateParams,
) -> Union[FunctionInventionRecursive, Stream[FunctionInventionRecursiveChunk]]:
    """Create a recursive function invention.

    If ``params.stream`` is true, returns a streaming response.
    """
    if getattr(params, "stream", None):
        return await client.post_streaming(
            "functions/inventions/recursive", params,
        )
    return await client.post_unary("functions/inventions/recursive", params)
