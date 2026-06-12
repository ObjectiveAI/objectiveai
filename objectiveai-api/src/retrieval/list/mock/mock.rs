//! Mock list source implementation.

use crate::ctx;
use objectiveai_sdk::error::ResponseError;

pub struct MockClient;

#[async_trait::async_trait]
impl<CTXEXT> super::super::Client<CTXEXT> for MockClient
where
    CTXEXT: Send + Sync + 'static,
{
    async fn list_agents<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<CTXEXT, PC>,
    ) -> Result<objectiveai_sdk::agent::response::ListAgentResponse, ResponseError> {
        Ok(crate::mock::list_agents())
    }

    async fn list_swarms<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<CTXEXT, PC>,
    ) -> Result<objectiveai_sdk::swarm::response::ListSwarmResponse, ResponseError> {
        Ok(crate::mock::list_swarms())
    }

    async fn list_functions<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<CTXEXT, PC>,
    ) -> Result<objectiveai_sdk::functions::response::ListFunctionResponse, ResponseError> {
        Ok(crate::mock::list_functions())
    }

    async fn list_profiles<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<CTXEXT, PC>,
    ) -> Result<objectiveai_sdk::functions::profiles::response::ListProfileResponse, ResponseError> {
        Ok(crate::mock::list_profiles())
    }

    async fn list_function_profile_pairs<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<CTXEXT, PC>,
    ) -> Result<objectiveai_sdk::functions::response::ListFunctionProfilePairResponse, ResponseError>
    {
        unimplemented!("Mock does not support listing function-profile pairs")
    }
}
