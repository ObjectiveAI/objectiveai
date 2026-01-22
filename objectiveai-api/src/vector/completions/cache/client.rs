use crate::{ctx, vector};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Client<CTXEXT, FVVOTE, FCVOTE> {
    pub completion_votes_fetcher: Arc<FVVOTE>,
    pub cache_vote_fetcher: Arc<FCVOTE>,
    _marker: std::marker::PhantomData<CTXEXT>,
}

impl<CTXEXT, FVVOTE, FCVOTE> Client<CTXEXT, FVVOTE, FCVOTE> {
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

    pub async fn fetch_cache_vote(
        &self,
        ctx: ctx::Context<CTXEXT>,
        model: &str,
        models: Option<&[&str]>,
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
