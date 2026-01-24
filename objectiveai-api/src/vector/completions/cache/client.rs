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
        ctx: ctx::Context<CTXEXT>,
        id: &str,
    ) -> Result<
        objectiveai::vector::completions::cache::response::CompletionVotes,
        objectiveai::error::ResponseError,
    > {
        let data = self.completion_votes_fetcher.fetch(ctx, id).await?;
        Ok(objectiveai::vector::completions::cache::response::CompletionVotes {
            data,
        })
    }

    /// Requests a vote from the global ObjectiveAI cache.
    ///
    /// Returns a cached vote if one exists for the given model, messages, tools, and responses.
    pub async fn fetch_cache_vote(
        &self,
        ctx: ctx::Context<CTXEXT>,
        model: &objectiveai::chat::completions::request::Model,
        models: Option<&[objectiveai::chat::completions::request::Model]>,
        messages: &[objectiveai::chat::completions::request::Message],
        tools: Option<&[objectiveai::chat::completions::request::Tool]>,
        responses: &[objectiveai::chat::completions::request::RichContent],
    ) -> Result<
        objectiveai::vector::completions::cache::response::CacheVote,
        objectiveai::error::ResponseError,
    > {
        let vote = self
            .cache_vote_fetcher
            .fetch(ctx, model, models, messages, tools, responses)
            .await?;
        Ok(
            objectiveai::vector::completions::cache::response::CacheVote {
                vote,
            },
        )
    }
}
