"""HTTP functions for laboratory executions."""

from __future__ import annotations

from typing import TYPE_CHECKING, Union

if TYPE_CHECKING:
    from objectiveai.client import ObjectiveAI
    from objectiveai.laboratories.executions.request import LaboratoryExecutionCreateParams
    from objectiveai.laboratories.executions.response.streaming import LaboratoryExecutionChunk
    from objectiveai.laboratories.executions.response.unary import LaboratoryExecution
    from objectiveai.stream import Stream


async def create_laboratory_execution(
    client: ObjectiveAI,
    params: LaboratoryExecutionCreateParams,
) -> Union[LaboratoryExecution, Stream[LaboratoryExecutionChunk]]:
    """Execute a laboratory.

    If ``params.stream`` is true, returns a streaming response.
    """
    if getattr(params, "stream", None):
        return await client.post_streaming("laboratories/executions", params)
    return await client.post_unary("laboratories/executions", params)
