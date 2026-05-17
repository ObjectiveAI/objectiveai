"""HTTP functions for authentication endpoints."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from objectiveai_sdk.auth import (
        ApiKeyWithMetadata,
        CreateApiKeyRequest,
        CreateOpenRouterByokApiKeyRequest,
        DisableApiKeyRequest,
        GetCreditsResponse,
        GetOpenRouterByokApiKeyResponse,
        ListApiKeyResponse,
    )
    from objectiveai_sdk.client import ObjectiveAI


async def create_api_key(
    client: ObjectiveAI, body: CreateApiKeyRequest,
) -> ApiKeyWithMetadata:
    """Create a new API key."""
    return await client.post_unary("auth/keys", body)


async def create_openrouter_byok_api_key(
    client: ObjectiveAI, body: CreateOpenRouterByokApiKeyRequest,
) -> GetOpenRouterByokApiKeyResponse:
    """Create or update the OpenRouter BYOK API key."""
    return await client.post_unary("auth/keys/openrouter", body)


async def disable_api_key(
    client: ObjectiveAI, body: DisableApiKeyRequest,
) -> ApiKeyWithMetadata:
    """Disable an existing API key."""
    return await client.delete_unary("auth/keys", body)


async def delete_openrouter_byok_api_key(client: ObjectiveAI) -> None:
    """Delete the OpenRouter BYOK API key."""
    return await client.delete_unary("auth/keys/openrouter")


async def list_api_keys(client: ObjectiveAI) -> ListApiKeyResponse:
    """List all API keys for the authenticated user."""
    return await client.get_unary("auth/keys")


async def get_openrouter_byok_api_key(
    client: ObjectiveAI,
) -> GetOpenRouterByokApiKeyResponse:
    """Retrieve the configured OpenRouter BYOK API key."""
    return await client.get_unary("auth/keys/openrouter")


async def get_credits(client: ObjectiveAI) -> GetCreditsResponse:
    """Retrieve the user's credit balance."""
    return await client.get_unary("auth/credits")
