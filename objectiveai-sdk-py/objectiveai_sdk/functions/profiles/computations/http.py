"""HTTP functions for profile computation endpoints."""

from __future__ import annotations

from typing import TYPE_CHECKING, Union

if TYPE_CHECKING:
    from objectiveai.client import ObjectiveAI
    from objectiveai.functions.profiles.computations.request import (
        FunctionProfileComputationCreateParams,
    )
    from objectiveai.functions.profiles.computations.response.streaming import (
        FunctionProfileComputationChunk,
    )
    from objectiveai.functions.profiles.computations.response.unary import (
        FunctionProfileComputation,
    )
    from objectiveai.stream import Stream


async def compute_profile(
    client: ObjectiveAI,
    params: FunctionProfileComputationCreateParams,
) -> Union[FunctionProfileComputation, Stream[FunctionProfileComputationChunk]]:
    """Compute a profile for a function.

    If ``params.stream`` is true, returns a streaming response.
    """
    if getattr(params, "stream", None):
        return await client.post_streaming("functions/profiles/compute", params)
    return await client.post_unary("functions/profiles/compute", params)
