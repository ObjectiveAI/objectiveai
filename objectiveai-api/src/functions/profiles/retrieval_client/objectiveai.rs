//! ObjectiveAI API implementation of the Profile retrieval client.

use crate::ctx;
use std::sync::Arc;

/// Lists Profiles and retrieves usage via the ObjectiveAI API.
pub struct ObjectiveAiClient {
    /// The HTTP client for API requests.
    pub client: Arc<objectiveai::HttpClient>,
}

impl ObjectiveAiClient {
    /// Creates a new ObjectiveAI Profile retrieval client.
    pub fn new(client: Arc<objectiveai::HttpClient>) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl<CTXEXT> super::Client<CTXEXT> for ObjectiveAiClient
where
    CTXEXT: Send + Sync + 'static,
{
    async fn list_profiles(
        &self,
        _ctx: ctx::Context<CTXEXT>,
    ) -> Result<
        objectiveai::functions::profiles::response::ListProfile,
        objectiveai::error::ResponseError,
    > {
        objectiveai::functions::profiles::list_profiles(&self.client)
            .await
            .map_err(|e| objectiveai::error::ResponseError::from(&e))
    }

    async fn get_profile_usage(
        &self,
        _ctx: ctx::Context<CTXEXT>,
        owner: &str,
        repository: &str,
        commit: Option<&str>,
    ) -> Result<
        objectiveai::functions::profiles::response::UsageProfile,
        objectiveai::error::ResponseError,
    > {
        objectiveai::functions::profiles::get_profile_usage(&self.client, owner, repository, commit)
            .await
            .map_err(|e| objectiveai::error::ResponseError::from(&e))
    }
}
