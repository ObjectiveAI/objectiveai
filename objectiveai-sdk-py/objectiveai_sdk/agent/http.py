"""HTTP functions for agent endpoints."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from objectiveai_sdk.agent import (
        GetAgentResponse,
        ListAgentResponse,
        ListAgentsRequest,
        GetAgentRequest,
        UsageAgentResponse,
    )
    from objectiveai_sdk.client import ObjectiveAI


async def list_agents(
    client: ObjectiveAI, params: ListAgentsRequest,
) -> ListAgentResponse:
    """List all agents that have been used."""
    return await client.post_unary("agents/list", params)


async def get_agent(
    client: ObjectiveAI, params: GetAgentRequest,
) -> GetAgentResponse:
    """Retrieve a specific agent."""
    return await client.post_unary("agents", params)


async def get_agent_usage(
    client: ObjectiveAI, params: GetAgentRequest,
) -> UsageAgentResponse:
    """Retrieve usage statistics for a specific agent."""
    return await client.post_unary("agents/usage", params)
