//! Filesystem list source implementation.

use crate::ctx;
use objectiveai_sdk::error::ResponseError;
use std::sync::Arc;

pub struct FilesystemClient {
    pub client: Arc<crate::filesystem::Client>,
}

impl FilesystemClient {
    pub fn new(client: Arc<crate::filesystem::Client>) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl<CTXEXT> super::super::Client<CTXEXT> for FilesystemClient
where
    CTXEXT: Send + Sync + 'static,
{
    async fn list_agents<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<CTXEXT, PC>,
    ) -> Result<objectiveai_sdk::agent::response::ListAgentResponse, ResponseError> {
        Ok(objectiveai_sdk::agent::response::ListAgentResponse { data: vec![] })
    }

    async fn list_swarms<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<CTXEXT, PC>,
    ) -> Result<objectiveai_sdk::swarm::response::ListSwarmResponse, ResponseError> {
        Ok(objectiveai_sdk::swarm::response::ListSwarmResponse { data: vec![] })
    }

    async fn list_functions<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<CTXEXT, PC>,
    ) -> Result<objectiveai_sdk::functions::response::ListFunctionResponse, ResponseError> {
        Ok(objectiveai_sdk::functions::response::ListFunctionResponse { data: vec![] })
    }

    async fn list_profiles<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<CTXEXT, PC>,
    ) -> Result<objectiveai_sdk::functions::profiles::response::ListProfileResponse, ResponseError> {
        Ok(objectiveai_sdk::functions::profiles::response::ListProfileResponse { data: vec![] })
    }

    async fn list_function_profile_pairs<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<CTXEXT, PC>,
    ) -> Result<objectiveai_sdk::functions::response::ListFunctionProfilePairResponse, ResponseError>
    {
        unimplemented!("Filesystem does not support listing function-profile pairs")
    }
}
