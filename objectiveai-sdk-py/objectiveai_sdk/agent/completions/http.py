"""HTTP functions for agent completions."""

from __future__ import annotations

from typing import TYPE_CHECKING, Union

if TYPE_CHECKING:
    from objectiveai_sdk.agent.completions.request import AgentCompletionCreateParams
    from objectiveai_sdk.agent.completions.request.agent_completion_notify_params import (
        AgentCompletionNotifyParams,
    )
    from objectiveai_sdk.agent.completions.response.streaming import AgentCompletionChunk
    from objectiveai_sdk.agent.completions.response.unary import AgentCompletion
    from objectiveai_sdk.client import ObjectiveAI
    from objectiveai_sdk.stream import Stream


async def create_agent_completion(
    client: ObjectiveAI,
    params: AgentCompletionCreateParams,
) -> Union[AgentCompletion, Stream[AgentCompletionChunk]]:
    """Create an agent completion.

    If ``params.stream`` is true, returns a streaming response.
    Otherwise returns the complete response.
    """
    if getattr(params, "stream", None):
        return await client.post_streaming("agent/completions", params)
    return await client.post_unary("agent/completions", params)


async def notify_agent_completion(
    client: ObjectiveAI,
    params: AgentCompletionNotifyParams,
) -> None:
    """Notify a running agent completion with a user message.

    Pushes a ``RichContent`` payload at the agent completion identified
    by ``params.response_id``; the api queues it and surfaces it to
    the model on its next natural inspection point. There is no
    response body — any 2xx status is the success signal.
    """
    await client.post_unary_no_response("agent/completions/notify", params)
