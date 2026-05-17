"""HTTP functions for invention state endpoints."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from objectiveai_sdk.client import ObjectiveAI
    from objectiveai_sdk.functions.inventions.state import GetFunctionInventionStateResponse
    from objectiveai_sdk.remote_path_commit_optional import RemotePathCommitOptional


async def get_function_invention_state(
    client: ObjectiveAI, params: RemotePathCommitOptional,
) -> GetFunctionInventionStateResponse:
    """Retrieve a function invention state definition."""
    return await client.post_unary("functions/inventions/state", params)
