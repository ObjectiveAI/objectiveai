//! Vote cache client implementation.

use crate::{ctx, vector};
use std::sync::Arc;

/// Client for retrieving cached votes.
#[derive(Debug, Clone)]
pub struct Client<CTXEXT, FVVOTE, FCVOTE> {
    /// Fetcher for votes from historical completions.
    pub completion_votes_fetcher: Arc<FVVOTE>,
    /// Fetcher for votes from the global cache.
    pub cache_vote_fetcher: Arc<FCVOTE>,
    _marker: std::marker::PhantomData<CTXEXT>,
}

impl<CTXEXT, FVVOTE, FCVOTE> Client<CTXEXT, FVVOTE, FCVOTE> {
    /// Creates a new cache client.
    pub fn new(
        completion_votes_fetcher: Arc<FVVOTE>,
        cache_vote_fetcher: Arc<FCVOTE>,
    ) -> Self {
        Self {
            completion_votes_fetcher,
            cache_vote_fetcher,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<CTXEXT, FVVOTE, FCVOTE> Client<CTXEXT, FVVOTE, FCVOTE>
where
    CTXEXT: Send + Sync + 'static,
    FVVOTE: vector::completions::completion_votes_fetcher::Fetcher<CTXEXT>
        + Send
        + Sync
        + 'static,
    FCVOTE: vector::completions::cache_vote_fetcher::Fetcher<CTXEXT>
        + Send
        + Sync
        + 'static,
{
    /// Retrieves all votes from a historical vector completion.
    pub async fn fetch_completion_votes(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        id: &str,
    ) -> Result<
        objectiveai_sdk::vector::completions::cache::response::CompletionVotes,
        objectiveai_sdk::error::ResponseError,
    > {
        let data = self.completion_votes_fetcher.fetch(ctx, id).await?;
        Ok(objectiveai_sdk::vector::completions::cache::response::CompletionVotes {
            data,
        })
    }

    /// Requests a vote from the global ObjectiveAI cache.
    ///
    /// Returns a cached vote if one exists for the given model, messages, tools, and responses.
    pub async fn fetch_cache_vote(
        &self,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        agent: &objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemote,
        messages: &[objectiveai_sdk::agent::completions::message::Message],
        responses: &[objectiveai_sdk::agent::completions::message::RichContent],
    ) -> Result<
        objectiveai_sdk::vector::completions::cache::response::CacheVote,
        objectiveai_sdk::error::ResponseError,
    > {
        let vote = self
            .cache_vote_fetcher
            .fetch(ctx, agent, messages, responses)
            .await?;
        Ok(
            objectiveai_sdk::vector::completions::cache::response::CacheVote {
                vote,
            },
        )
    }
}
