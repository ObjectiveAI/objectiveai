//! GitHub fetch source implementation.

use crate::ctx;
use objectiveai_sdk::error::ResponseError;
use std::sync::Arc;

pub struct GithubClient {
    pub client: Arc<crate::github::Client>,
}

impl GithubClient {
    pub fn new(client: Arc<crate::github::Client>) -> Self {
        Self { client }
    }
}

/// Extracts (owner, repository, commit) from a RemotePath.
/// Panics on Mock variant (mock paths should not reach the GitHub client).
fn github_fields(path: &objectiveai_sdk::RemotePath) -> (&str, &str, &str) {
    match path {
        objectiveai_sdk::RemotePath::Github { owner, repository, commit }
        | objectiveai_sdk::RemotePath::Filesystem { owner, repository, commit } => {
            (owner, repository, commit)
        }
        objectiveai_sdk::RemotePath::Mock { .. } => {
            unreachable!("mock paths should not reach the GitHub client")
        }
    }
}

#[async_trait::async_trait]
impl<CTXEXT> super::super::Client<CTXEXT> for GithubClient
where
    CTXEXT: Send + Sync + 'static + ctx::ContextExt,
{
    async fn get_agent<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks>, ResponseError> {
        let (owner, repository, commit) = github_fields(path);
        self.client
            .read_json(ctx, owner, repository, commit, "agent.json")
            .await
            .map_err(|e| ResponseError::from(&e))
    }

    async fn get_swarm<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::swarm::RemoteSwarmBase>, ResponseError> {
        let (owner, repository, commit) = github_fields(path);
        self.client
            .read_json(ctx, owner, repository, commit, "swarm.json")
            .await
            .map_err(|e| ResponseError::from(&e))
    }

    async fn get_function<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::functions::FullRemoteFunction>, ResponseError> {
        let (owner, repository, commit) = github_fields(path);
        self.client
            .read_json(ctx, owner, repository, commit, "function.json")
            .await
            .map_err(|e| ResponseError::from(&e))
    }

    async fn get_profile<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::functions::RemoteProfile>, ResponseError> {
        let (owner, repository, commit) = github_fields(path);
        self.client
            .read_json(ctx, owner, repository, commit, "profile.json")
            .await
            .map_err(|e| ResponseError::from(&e))
    }

    async fn get_prompt<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::functions::inventions::prompts::RemotePrompt>, ResponseError> {
        let (owner, repository, commit) = github_fields(path);
        self.client
            .read_json(ctx, owner, repository, commit, "prompt.json")
            .await
            .map_err(|e| ResponseError::from(&e))
    }

    async fn get_function_invention_state_file<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai_sdk::RemotePath,
        filename: &'static str,
    ) -> Result<Option<String>, ResponseError> {
        let (owner, repository, commit) = github_fields(path);
        self.client
            .read_file(ctx, owner, repository, commit, filename)
            .await
            .map_err(|e| ResponseError::from(&e))
    }

    async fn resolve_latest<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        ctx: &ctx::Context<CTXEXT, PC>,
        _kind: crate::retrieval::Kind,
        path: &objectiveai_sdk::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai_sdk::RemotePath>, ResponseError> {
        match path {
            objectiveai_sdk::RemotePathCommitOptional::Github { owner, repository, commit } => {
                let resolved_commit = match commit {
                    Some(c) => c.clone(),
                    None => {
                        match self.client.fetch_latest_commit(ctx, owner, repository).await {
                            Ok(Some(c)) => c,
                            Ok(None) => return Ok(None),
                            Err(e) => return Err(ResponseError::from(&e)),
                        }
                    }
                };
                Ok(Some(objectiveai_sdk::RemotePath::Github {
                    owner: owner.clone(),
                    repository: repository.clone(),
                    commit: resolved_commit,
                }))
            }
            _ => Ok(None),
        }
    }
}
