"""HTTP functions for function management endpoints."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from objectiveai_sdk.client import ObjectiveAI
    from objectiveai_sdk.functions import (
        GetFunctionProfilePairUsageRequest,
        GetFunctionRequest,
        GetFunctionResponse,
        ListFunctionProfilePairResponse,
        ListFunctionProfilePairsRequest,
        ListFunctionResponse,
        ListFunctionsRequest,
        UsageFunctionProfilePairResponse,
        UsageFunctionResponse,
    )


async def list_functions(
    client: ObjectiveAI, params: ListFunctionsRequest,
) -> ListFunctionResponse:
    """List all functions accessible to the authenticated user."""
    return await client.post_unary("functions/list", params)


async def get_function(
    client: ObjectiveAI, params: GetFunctionRequest,
) -> GetFunctionResponse:
    """Retrieve a function definition."""
    return await client.post_unary("functions", params)


async def get_function_usage(
    client: ObjectiveAI, params: GetFunctionRequest,
) -> UsageFunctionResponse:
    """Retrieve usage statistics for a specific function."""
    return await client.post_unary("functions/usage", params)


async def list_function_profile_pairs(
    client: ObjectiveAI, params: ListFunctionProfilePairsRequest,
) -> ListFunctionProfilePairResponse:
    """List all function-profile pairs accessible to the authenticated user."""
    return await client.post_unary("functions/profiles/pairs/list", params)


async def get_function_profile_pair_usage(
    client: ObjectiveAI, params: GetFunctionProfilePairUsageRequest,
) -> UsageFunctionProfilePairResponse:
    """Retrieve usage statistics for a specific function-profile pair."""
    return await client.post_unary("functions/profiles/pairs/usage", params)
