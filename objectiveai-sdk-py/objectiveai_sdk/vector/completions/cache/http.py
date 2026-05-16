"""HTTP functions for vector completion cache endpoints."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from objectiveai.client import ObjectiveAI
    from objectiveai.vector.completions.cache import (
        CacheVote,
        CacheVoteRequest,
        CompletionVotes,
        GetCompletionVotesRequest,
    )


async def get_completion_votes(
    client: ObjectiveAI, params: GetCompletionVotesRequest,
) -> CompletionVotes:
    """Retrieve votes for a specific vector completion."""
    return await client.post_unary("vector/completions/votes", params)


async def get_cache_vote(
    client: ObjectiveAI, params: CacheVoteRequest,
) -> CacheVote:
    """Retrieve a cached vote."""
    return await client.post_unary("vector/completions/cache", params)
