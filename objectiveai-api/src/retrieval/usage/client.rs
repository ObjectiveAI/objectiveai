//! Usage trait — only ObjectiveAI tracks usage statistics.

use crate::ctx;
use objectiveai_sdk::error::ResponseError;

/// Retrieves usage statistics for resources.
///
/// Only ObjectiveAI implements this.
#[async_trait::async_trait]
pub trait Client<CTXEXT>: Send + Sync + 'static {
    async fn get_agent_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai_sdk::agent::request::GetAgentRequest,
    ) -> Result<objectiveai_sdk::agent::response::UsageAgentResponse, ResponseError>;

    async fn get_swarm_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai_sdk::swarm::request::GetSwarmRequest,
    ) -> Result<objectiveai_sdk::swarm::response::UsageSwarmResponse, ResponseError>;

    async fn get_function_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai_sdk::functions::request::GetFunctionRequest,
    ) -> Result<objectiveai_sdk::functions::response::UsageFunctionResponse, ResponseError>;

    async fn get_profile_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai_sdk::functions::profiles::request::GetProfileRequest,
    ) -> Result<objectiveai_sdk::functions::profiles::response::UsageProfileResponse, ResponseError>;

    async fn get_prompt_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai_sdk::functions::inventions::prompts::request::GetPromptRequest,
    ) -> Result<objectiveai_sdk::functions::inventions::prompts::response::UsagePromptResponse, ResponseError>;

    async fn get_function_profile_pair_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai_sdk::functions::request::GetFunctionProfilePairUsageRequest,
    ) -> Result<objectiveai_sdk::functions::response::UsageFunctionProfilePairResponse, ResponseError>;
}
