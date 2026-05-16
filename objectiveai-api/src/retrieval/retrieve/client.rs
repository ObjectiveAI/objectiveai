//! FetchSource trait — implemented by Mock, Filesystem, and GitHub.

use crate::ctx;
use objectiveai_sdk::error::ResponseError;

/// A source that can fetch individual resource definitions by path.
///
/// Implemented by Mock, Filesystem, and GitHub.
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

    async fn get_prompt<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::functions::inventions::prompts::RemotePrompt>, ResponseError>;

    /// Reads a function invention state file by name from the resource at the given path.
    /// Kind is always Functions. Returns `None` if the file does not exist.
    async fn get_function_invention_state_file<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai_sdk::RemotePath,
        filename: &'static str,
    ) -> Result<Option<String>, ResponseError>;

    /// Resolves a `RemotePathCommitOptional` to a full `RemotePath`.
    /// For sources with commits (Github, Filesystem), resolves the latest commit if missing.
    /// For Mock, returns a `RemotePath::Mock` directly.
    async fn resolve_latest<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        kind: crate::retrieval::Kind,
        path: &objectiveai_sdk::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai_sdk::RemotePath>, ResponseError>;
}
