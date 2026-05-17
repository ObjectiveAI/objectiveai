"""HTTP functions for swarm endpoints."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from objectiveai_sdk.client import ObjectiveAI
    from objectiveai_sdk.swarm import (
        GetSwarmRequest,
        GetSwarmResponse,
        ListSwarmResponse,
        ListSwarmsRequest,
        UsageSwarmResponse,
    )


async def list_swarms(
    client: ObjectiveAI, params: ListSwarmsRequest,
) -> ListSwarmResponse:
    """List all swarms that have been used."""
    return await client.post_unary("swarms/list", params)


async def get_swarm(
    client: ObjectiveAI, params: GetSwarmRequest,
) -> GetSwarmResponse:
    """Retrieve a specific swarm."""
    return await client.post_unary("swarms", params)


async def get_swarm_usage(
    client: ObjectiveAI, params: GetSwarmRequest,
) -> UsageSwarmResponse:
    """Retrieve usage statistics for a specific swarm."""
    return await client.post_unary("swarms/usage", params)
