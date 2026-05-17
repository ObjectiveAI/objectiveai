"""HTTP functions for the error endpoint."""

from __future__ import annotations

from typing import TYPE_CHECKING, Union

if TYPE_CHECKING:
    from objectiveai_sdk.client import ObjectiveAI
    from objectiveai_sdk.error import ErrorCreateParams, ErrorResponse
    from objectiveai_sdk.stream import Stream


async def create_error(
    client: ObjectiveAI,
    params: ErrorCreateParams,
) -> Union[ErrorResponse, Stream[ErrorResponse]]:
    """Create an error response.

    If ``params.stream`` is true, returns a streaming response.
    """
    if getattr(params, "stream", None):
        return await client.post_streaming("error", params)
    return await client.post_unary("error", params)
