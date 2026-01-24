//! ObjectiveAI API implementation of the cache vote fetcher.

use crate::ctx;
use objectiveai::error::StatusError;
use std::sync::Arc;

/// Fetches cached votes from the ObjectiveAI API.
pub struct ObjectiveAiFetcher {
    /// The HTTP client for API requests.
    pub client: Arc<objectiveai::HttpClient>,
}

impl ObjectiveAiFetcher {
    /// Creates a new ObjectiveAI cache vote fetcher.
    pub fn new(client: Arc<objectiveai::HttpClient>) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl<CTXEXT> super::Fetcher<CTXEXT> for ObjectiveAiFetcher
where
    CTXEXT: Send + Sync + 'static,
{
    async fn fetch(
        &self,
        _ctx: ctx::Context<CTXEXT>,
        model: &objectiveai::chat::completions::request::Model,
        models: Option<&[objectiveai::chat::completions::request::Model]>,
        messages: &[objectiveai::chat::completions::request::Message],
        tools: Option<&[objectiveai::chat::completions::request::Tool]>,
        responses: &[objectiveai::chat::completions::request::RichContent],
    ) -> Result<
        Option<objectiveai::vector::completions::response::Vote>,
        objectiveai::error::ResponseError,
    > {
        let request = objectiveai::vector::completions::cache::request::CacheVoteRequest::Ref(
            objectiveai::vector::completions::cache::request::CacheVoteRequestRef {
                model,
                models,
                messages,
                tools,
                responses,
            },
        );
        match objectiveai::vector::completions::cache::get_cache_vote(
            &self.client,
            &request,
        )
        .await
        {
            Ok(vote) => Ok(vote.vote),
            Err(e) if e.status() == 404 => Ok(None),
            Err(e) => Err(objectiveai::error::ResponseError::from(&e)),
        }
    }
}
