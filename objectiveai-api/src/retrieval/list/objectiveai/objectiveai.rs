//! ObjectiveAI list source implementation.

use crate::ctx;
use objectiveai_sdk::error::ResponseError;
use std::sync::Arc;

pub struct ObjectiveAiClient {
    pub client: Arc<crate::objectiveai_http::Client>,
}

impl ObjectiveAiClient {
    pub fn new(client: Arc<crate::objectiveai_http::Client>) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl<CTXEXT> super::super::Client<CTXEXT> for ObjectiveAiClient
where
    CTXEXT: Send + Sync + 'static + ctx::ContextExt,
{
    async fn list_agents<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
    ) -> Result<objectiveai_sdk::agent::response::ListAgentResponse, ResponseError> {
        let client = self.client.with_authorization(ctx).await;
        objectiveai_sdk::agent::list_agents(
            &client,
            objectiveai_sdk::agent::request::ListAgentsRequest {
                source: Some(objectiveai_sdk::agent::request::ListAgentsSource::Objectiveai),
            },
        )
        .await
        .map_err(|e| ResponseError::from(&e))
    }

    async fn list_swarms<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
    ) -> Result<objectiveai_sdk::swarm::response::ListSwarmResponse, ResponseError> {
        let client = self.client.with_authorization(ctx).await;
        objectiveai_sdk::swarm::list_swarms(
            &client,
            objectiveai_sdk::swarm::request::ListSwarmsRequest {
                source: Some(objectiveai_sdk::swarm::request::ListSwarmsSource::Objectiveai),
            },
        )
        .await
        .map_err(|e| ResponseError::from(&e))
    }

    async fn list_functions<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
    ) -> Result<objectiveai_sdk::functions::response::ListFunctionResponse, ResponseError> {
        let client = self.client.with_authorization(ctx).await;
        objectiveai_sdk::functions::list_functions(
            &client,
            objectiveai_sdk::functions::request::ListFunctionsRequest {
                source: Some(objectiveai_sdk::functions::request::ListFunctionsSource::Objectiveai),
            },
        )
        .await
        .map_err(|e| ResponseError::from(&e))
    }

    async fn list_profiles<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
    ) -> Result<objectiveai_sdk::functions::profiles::response::ListProfileResponse, ResponseError>
    {
        let client = self.client.with_authorization(ctx).await;
        objectiveai_sdk::functions::profiles::list_profiles(
            &client,
            objectiveai_sdk::functions::profiles::request::ListProfilesRequest {
                source: Some(
                    objectiveai_sdk::functions::profiles::request::ListProfilesSource::Objectiveai,
                ),
            },
        )
        .await
        .map_err(|e| ResponseError::from(&e))
    }

    async fn list_function_profile_pairs<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
    ) -> Result<objectiveai_sdk::functions::response::ListFunctionProfilePairResponse, ResponseError>
    {
        let client = self.client.with_authorization(ctx).await;
        objectiveai_sdk::functions::list_function_profile_pairs(
            &client,
            objectiveai_sdk::functions::request::ListFunctionProfilePairsRequest {
                source: Some(
                    objectiveai_sdk::functions::request::ListFunctionProfilePairsSource::Objectiveai,
                ),
            },
        )
        .await
        .map_err(|e| ResponseError::from(&e))
    }
}
