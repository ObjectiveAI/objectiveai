//! ObjectiveAI usage statistics implementation.

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
    async fn get_agent_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai_sdk::agent::request::GetAgentRequest,
    ) -> Result<objectiveai_sdk::agent::response::UsageAgentResponse, ResponseError> {
        let client = self.objectiveai_client(ctx).await;
        objectiveai_sdk::agent::get_agent_usage(&client, params.clone())
            .await
            .map_err(|e| ResponseError::from(&e))
    }

    async fn get_swarm_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai_sdk::swarm::request::GetSwarmRequest,
    ) -> Result<objectiveai_sdk::swarm::response::UsageSwarmResponse, ResponseError> {
        let client = self.objectiveai_client(ctx).await;
        objectiveai_sdk::swarm::get_swarm_usage(&client, params.clone())
            .await
            .map_err(|e| ResponseError::from(&e))
    }

    async fn get_function_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai_sdk::functions::request::GetFunctionRequest,
    ) -> Result<objectiveai_sdk::functions::response::UsageFunctionResponse, ResponseError> {
        let client = self.objectiveai_client(ctx).await;
        objectiveai_sdk::functions::get_function_usage(&client, params.clone())
            .await
            .map_err(|e| ResponseError::from(&e))
    }

    async fn get_profile_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai_sdk::functions::profiles::request::GetProfileRequest,
    ) -> Result<objectiveai_sdk::functions::profiles::response::UsageProfileResponse, ResponseError>
    {
        let client = self.objectiveai_client(ctx).await;
        objectiveai_sdk::functions::profiles::get_profile_usage(&client, params.clone())
            .await
            .map_err(|e| ResponseError::from(&e))
    }

    async fn get_function_profile_pair_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai_sdk::functions::request::GetFunctionProfilePairUsageRequest,
    ) -> Result<objectiveai_sdk::functions::response::UsageFunctionProfilePairResponse, ResponseError>
    {
        let client = self.objectiveai_client(ctx).await;
        objectiveai_sdk::functions::get_function_profile_pair_usage(&client, params.clone())
            .await
            .map_err(|e| ResponseError::from(&e))
    }
}

impl ObjectiveAiClient {
    async fn objectiveai_client<CTXEXT: ctx::ContextExt, PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
    ) -> objectiveai_sdk::HttpClient {
        self.client.with_authorization(ctx).await
    }
}
