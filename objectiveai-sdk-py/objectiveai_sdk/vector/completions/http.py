"""HTTP functions for vector completion endpoints."""

from __future__ import annotations

from typing import TYPE_CHECKING, Union

if TYPE_CHECKING:
    from objectiveai_sdk.client import ObjectiveAI
    from objectiveai_sdk.stream import Stream
    from objectiveai_sdk.vector.completions.request import VectorCompletionCreateParams
    from objectiveai_sdk.vector.completions.response.streaming import VectorCompletionChunk
    from objectiveai_sdk.vector.completions.response.unary import VectorCompletion


async def create_vector_completion(
    client: ObjectiveAI,
    params: VectorCompletionCreateParams,
) -> Union[VectorCompletion, Stream[VectorCompletionChunk]]:
    """Create a vector completion.

    If ``params.stream`` is true, returns a streaming response.
    Otherwise returns the complete response.
    """
    if getattr(params, "stream", None):
        return await client.post_streaming("vector/completions", params)
    return await client.post_unary("vector/completions", params)
