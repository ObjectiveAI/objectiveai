//! ObjectiveAI API implementation of the cache vote fetcher.

use crate::ctx;
use objectiveai_sdk::error::StatusError;
use std::sync::Arc;

/// Fetches cached votes from the ObjectiveAI API.
pub struct ObjectiveAiFetcher {
    /// The HTTP client for API requests.
    pub client: Arc<crate::objectiveai_http::Client>,
}

impl ObjectiveAiFetcher {
    /// Creates a new ObjectiveAI cache vote fetcher.
    pub fn new(client: Arc<crate::objectiveai_http::Client>) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl<CTXEXT> super::Fetcher<CTXEXT> for ObjectiveAiFetcher
where
    CTXEXT: Send + Sync + 'static + ctx::ContextExt,
{
    async fn fetch<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: ctx::Context<CTXEXT, PC>,
        agent: &objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemote,
        messages: &[objectiveai_sdk::agent::completions::message::Message],
        responses: &[objectiveai_sdk::agent::completions::message::RichContent],
    ) -> Result<
        Option<objectiveai_sdk::vector::completions::response::Vote>,
        objectiveai_sdk::error::ResponseError,
    > {
        let client = self.client.with_authorization(&ctx).await;
        let request = objectiveai_sdk::vector::completions::cache::request::CacheVoteRequest::Ref(
            objectiveai_sdk::vector::completions::cache::request::CacheVoteRequestRef {
                agent,
                messages,
                responses,
            },
        );
        match objectiveai_sdk::vector::completions::cache::get_cache_vote(
            &client,
            &request,
        )
        .await
        {
            Ok(vote) => Ok(vote.vote),
            Err(e) if e.status() == 404 => Ok(None),
            Err(e) => Err(objectiveai_sdk::error::ResponseError::from(&e)),
        }
    }
}
