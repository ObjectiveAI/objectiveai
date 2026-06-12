//! Usage router — delegates to the ObjectiveAI usage client.
//!
//! No caching needed for usage — these are simple pass-through requests.

use crate::ctx;
use objectiveai_sdk::error::ResponseError;
use std::sync::Arc;

/// Routes usage requests to the ObjectiveAI usage client.
///
/// Only ObjectiveAI tracks usage, so there's only one delegate.
pub struct Router<O, CTXEXT> {
    pub objectiveai: Arc<O>,
    _ctxext: std::marker::PhantomData<CTXEXT>,
}

impl<O, CTXEXT> Router<O, CTXEXT> {
    pub fn new(objectiveai: Arc<O>) -> Self {
        Self { objectiveai, _ctxext: std::marker::PhantomData }
    }
}

impl<O, CTXEXT> Router<O, CTXEXT>
where
    O: super::Client<CTXEXT>,
    CTXEXT: Send + Sync + 'static,
{
    pub async fn get_agent_usage(
        &self,
        ctx: &ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        params: &objectiveai_sdk::agent::request::GetAgentRequest,
    ) -> Result<objectiveai_sdk::agent::response::UsageAgentResponse, ResponseError> {
        self.objectiveai.get_agent_usage(ctx, params).await
    }

    pub async fn get_swarm_usage(
        &self,
        ctx: &ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        params: &objectiveai_sdk::swarm::request::GetSwarmRequest,
    ) -> Result<objectiveai_sdk::swarm::response::UsageSwarmResponse, ResponseError> {
        self.objectiveai.get_swarm_usage(ctx, params).await
    }

    pub async fn get_function_usage(
        &self,
        ctx: &ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        params: &objectiveai_sdk::functions::request::GetFunctionRequest,
    ) -> Result<objectiveai_sdk::functions::response::UsageFunctionResponse, ResponseError> {
        self.objectiveai.get_function_usage(ctx, params).await
    }

    pub async fn get_profile_usage(
        &self,
        ctx: &ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        params: &objectiveai_sdk::functions::profiles::request::GetProfileRequest,
    ) -> Result<objectiveai_sdk::functions::profiles::response::UsageProfileResponse, ResponseError> {
        self.objectiveai.get_profile_usage(ctx, params).await
    }

    pub async fn get_function_profile_pair_usage(
        &self,
        ctx: &ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        params: &objectiveai_sdk::functions::request::GetFunctionProfilePairUsageRequest,
    ) -> Result<objectiveai_sdk::functions::response::UsageFunctionProfilePairResponse, ResponseError>
    {
        self.objectiveai.get_function_profile_pair_usage(ctx, params).await
    }
}
