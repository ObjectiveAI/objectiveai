//! ObjectiveAI API implementation of the Profile computation client.

use crate::ctx;
use futures::{Stream, TryStreamExt};
use std::sync::Arc;

/// Computes Profiles via the ObjectiveAI API.
pub struct ObjectiveAiClient {
    /// The HTTP client for API requests.
    pub client: Arc<objectiveai::HttpClient>,
}

impl ObjectiveAiClient {
    /// Creates a new ObjectiveAI Profile computation client.
    pub fn new(client: Arc<objectiveai::HttpClient>) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl<CTXEXT> super::Client<CTXEXT> for ObjectiveAiClient
where
    CTXEXT: Send + Sync + 'static,
{
    async fn create_unary(
        &self,
        _ctx: ctx::Context<CTXEXT>,
        request: Arc<
            objectiveai::functions::profiles::computations::request::Request,
        >,
    ) -> Result<
        objectiveai::functions::profiles::computations::response::unary::FunctionProfileComputation,
        objectiveai::error::ResponseError,
    >{
        objectiveai::functions::profiles::computations::compute_profile_unary(
            &self.client,
            (*request).clone(),
        )
        .await
        .map_err(|e| objectiveai::error::ResponseError::from(&e))
    }

    async fn create_streaming(
        &self,
        _ctx: ctx::Context<CTXEXT>,
        request: Arc<
            objectiveai::functions::profiles::computations::request::Request,
        >,
    ) -> Result<
        impl Stream<Item = Result<
            objectiveai::functions::profiles::computations::response::streaming::FunctionProfileComputationChunk,
            objectiveai::error::ResponseError,
        >>
            + Send
            + 'static,
        objectiveai::error::ResponseError,
    >{
        let stream = objectiveai::functions::profiles::computations::compute_profile_streaming(
            &self.client,
            (*request).clone(),
        )
        .await
        .map_err(|e| objectiveai::error::ResponseError::from(&e))?;
        Ok(stream.map_err(|e| objectiveai::error::ResponseError::from(&e)))
    }
}
