//! Fetch-source trait — implemented by Mock, GitHub, and the
//! websocket-backed Client resolver.

use crate::ctx;
use objectiveai_sdk::error::ResponseError;

/// A source that can fetch individual resource definitions by path.
///
/// Implemented by Mock, GitHub, and the `Client` resolver (which asks
/// the connected client over the websocket reverse-channel).
/// ObjectiveAI API does NOT implement this (it proxies to GitHub).
#[async_trait::async_trait]
pub trait Client<CTXEXT>: Send + Sync + 'static {
    async fn get_agent<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks>, ResponseError>;

    async fn get_swarm<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::swarm::RemoteSwarmBase>, ResponseError>;

    async fn get_function<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::functions::FullRemoteFunction>, ResponseError>;

    async fn get_profile<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::functions::RemoteProfile>, ResponseError>;

    /// Resolves a `RemotePathCommitOptional` to a full `RemotePath`.
    /// For sources with commits (Github, Client), resolves the latest commit if missing.
    /// For Mock, returns a `RemotePath::Mock` directly.
    async fn resolve_latest<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        kind: crate::retrieval::Kind,
        path: &objectiveai_sdk::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai_sdk::RemotePath>, ResponseError>;
}
