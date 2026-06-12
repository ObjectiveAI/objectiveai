//! Mock fetch source implementation.

use crate::ctx;
use objectiveai_sdk::error::ResponseError;

pub struct MockClient;

#[async_trait::async_trait]
impl<CTXEXT> super::super::Client<CTXEXT> for MockClient
where
    CTXEXT: Send + Sync + 'static,
{
    async fn get_agent<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks>, ResponseError> {
        let name = match path {
            objectiveai_sdk::RemotePath::Mock { name } => name,
            _ => return Ok(None),
        };
        Ok(crate::mock::get_agent(name))
    }

    async fn get_swarm<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::swarm::RemoteSwarmBase>, ResponseError> {
        let name = match path {
            objectiveai_sdk::RemotePath::Mock { name } => name,
            _ => return Ok(None),
        };
        Ok(crate::mock::get_swarm(name))
    }

    async fn get_function<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::functions::FullRemoteFunction>, ResponseError> {
        let name = match path {
            objectiveai_sdk::RemotePath::Mock { name } => name,
            _ => return Ok(None),
        };
        Ok(crate::mock::get_function(name))
    }

    async fn get_profile<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::functions::RemoteProfile>, ResponseError> {
        let name = match path {
            objectiveai_sdk::RemotePath::Mock { name } => name,
            _ => return Ok(None),
        };
        Ok(crate::mock::get_profile(name))
    }

    async fn resolve_latest<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<CTXEXT, PC>,
        _kind: crate::retrieval::Kind,
        path: &objectiveai_sdk::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai_sdk::RemotePath>, ResponseError> {
        match path {
            objectiveai_sdk::RemotePathCommitOptional::Mock { name } => {
                Ok(Some(objectiveai_sdk::RemotePath::Mock { name: name.clone() }))
            }
            _ => Ok(None),
        }
    }
}
