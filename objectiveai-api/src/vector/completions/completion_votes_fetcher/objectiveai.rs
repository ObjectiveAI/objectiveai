//! ObjectiveAI API implementation of the completion votes fetcher.

use crate::ctx;
use objectiveai_sdk::error::StatusError;
use std::sync::Arc;

/// Fetches completion votes from the ObjectiveAI API.
pub struct ObjectiveAiFetcher {
    /// The HTTP client for API requests.
    pub client: Arc<crate::objectiveai_http::Client>,
}

impl ObjectiveAiFetcher {
    /// Creates a new ObjectiveAI completion votes fetcher.
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
        id: &str,
    ) -> Result<
        Option<Vec<objectiveai_sdk::vector::completions::response::Vote>>,
        objectiveai_sdk::error::ResponseError,
    > {
        let client = self.client.with_authorization(&ctx).await;
        let request = objectiveai_sdk::vector::completions::cache::request::GetCompletionVotesRequest {
            id: id.to_owned(),
        };
        match objectiveai_sdk::vector::completions::cache::get_completion_votes(
            &client,
            &request,
        )
        .await
        {
            Ok(votes) => Ok(votes.data),
            Err(e) if e.status() == 404 => Ok(None),
            Err(e) => Err(objectiveai_sdk::error::ResponseError::from(&e)),
        }
    }
}
