"""HTTP functions for invention prompt endpoints."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from objectiveai_sdk.client import ObjectiveAI
    from objectiveai_sdk.functions.inventions.prompts import (
        GetPromptResponse,
        ListPromptResponse,
        ListPromptsRequest,
        UsagePromptResponse,
    )
    from objectiveai_sdk.remote_path_commit_optional import RemotePathCommitOptional


async def list_prompts(
    client: ObjectiveAI, params: ListPromptsRequest,
) -> ListPromptResponse:
    """List all prompts accessible to the authenticated user."""
    return await client.post_unary("functions/inventions/prompts/list", params)


async def get_prompt(
    client: ObjectiveAI, params: RemotePathCommitOptional,
) -> GetPromptResponse:
    """Retrieve a prompt definition."""
    return await client.post_unary("functions/inventions/prompts", params)


async def get_prompt_usage(
    client: ObjectiveAI, params: RemotePathCommitOptional,
) -> UsagePromptResponse:
    """Retrieve usage statistics for a specific prompt."""
    return await client.post_unary("functions/inventions/prompts/usage", params)
