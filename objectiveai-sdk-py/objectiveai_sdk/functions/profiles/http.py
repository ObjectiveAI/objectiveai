"""HTTP functions for function profile endpoints."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from objectiveai_sdk.client import ObjectiveAI
    from objectiveai_sdk.functions.profiles import (
        GetProfileRequest,
        GetProfileResponse,
        ListProfileResponse,
        ListProfilesRequest,
        UsageProfileResponse,
    )


async def list_profiles(
    client: ObjectiveAI, params: ListProfilesRequest,
) -> ListProfileResponse:
    """List all profiles accessible to the authenticated user."""
    return await client.post_unary("functions/profiles/list", params)


async def get_profile(
    client: ObjectiveAI, params: GetProfileRequest,
) -> GetProfileResponse:
    """Retrieve a profile definition."""
    return await client.post_unary("functions/profiles", params)


async def get_profile_usage(
    client: ObjectiveAI, params: GetProfileRequest,
) -> UsageProfileResponse:
    """Retrieve usage statistics for a specific profile."""
    return await client.post_unary("functions/profiles/usage", params)
