//! Retrieve router — dispatches by `Remote`, resolves commits, and caches per request.

use crate::ctx;
use objectiveai::error::ResponseError;
use objectiveai::Remote;
use std::sync::Arc;

/// Routes fetch operations by `Remote` to GitHub/Filesystem/Mock,
/// with per-request deduplication caching via context caches.
///
/// Main methods accept `CommitOptional` enums (inline or remote ref).
/// If inline, converts directly. If remote, resolves commit, fetches
/// from source, converts, and returns the union type.
pub struct Router<G, F, M, CTXEXT> {
    pub github: Arc<G>,
    pub filesystem: Arc<F>,
    pub mock: Arc<M>,
    _ctxext: std::marker::PhantomData<CTXEXT>,
}

impl<G, F, M, CTXEXT> Router<G, F, M, CTXEXT> {
    pub fn new(github: Arc<G>, filesystem: Arc<F>, mock: Arc<M>) -> Self {
        Self { github, filesystem, mock, _ctxext: std::marker::PhantomData }
    }
}


impl<G, F, M, CTXEXT> Router<G, F, M, CTXEXT>
where
    G: super::Client<CTXEXT>,
    F: super::Client<CTXEXT>,
    M: super::Client<CTXEXT>,
    CTXEXT: Send + Sync + 'static,
{
    async fn dispatch_resolve_latest<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        remote: Remote,
        ctx: &ctx::Context<CTXEXT, PC>,
        kind: crate::retrieval::Kind,
        path: &objectiveai::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai::RemotePath>, ResponseError> {
        match remote {
            Remote::Github => self.github.resolve_latest(ctx, kind, path).await,
            Remote::Filesystem => self.filesystem.resolve_latest(ctx, kind, path).await,
            Remote::Mock => self.mock.resolve_latest(ctx, kind, path).await,
        }
    }

    async fn dispatch_get_agent<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        remote: Remote,
        ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai::RemotePath,
    ) -> Result<Option<objectiveai::agent::RemoteAgentBaseWithFallbacks>, ResponseError> {
        match remote {
            Remote::Github => self.github.get_agent(ctx, path).await,
            Remote::Filesystem => self.filesystem.get_agent(ctx, path).await,
            Remote::Mock => self.mock.get_agent(ctx, path).await,
        }
    }

    async fn dispatch_get_swarm<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        remote: Remote,
        ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai::RemotePath,
    ) -> Result<Option<objectiveai::swarm::RemoteSwarmBase>, ResponseError> {
        match remote {
            Remote::Github => self.github.get_swarm(ctx, path).await,
            Remote::Filesystem => self.filesystem.get_swarm(ctx, path).await,
            Remote::Mock => self.mock.get_swarm(ctx, path).await,
        }
    }

    async fn dispatch_get_function<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        remote: Remote,
        ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai::RemotePath,
    ) -> Result<Option<objectiveai::functions::FullRemoteFunction>, ResponseError> {
        match remote {
            Remote::Github => self.github.get_function(ctx, path).await,
            Remote::Filesystem => self.filesystem.get_function(ctx, path).await,
            Remote::Mock => self.mock.get_function(ctx, path).await,
        }
    }

    async fn dispatch_get_profile<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        remote: Remote,
        ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai::RemotePath,
    ) -> Result<Option<objectiveai::functions::RemoteProfile>, ResponseError> {
        match remote {
            Remote::Github => self.github.get_profile(ctx, path).await,
            Remote::Filesystem => self.filesystem.get_profile(ctx, path).await,
            Remote::Mock => self.mock.get_profile(ctx, path).await,
        }
    }

    async fn dispatch_get_prompt<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        remote: Remote,
        ctx: &ctx::Context<CTXEXT, PC>,
        path: &objectiveai::RemotePath,
    ) -> Result<Option<objectiveai::functions::inventions::prompts::RemotePrompt>, ResponseError> {
        match remote {
            Remote::Github => self.github.get_prompt(ctx, path).await,
            Remote::Filesystem => self.filesystem.get_prompt(ctx, path).await,
            Remote::Mock => self.mock.get_prompt(ctx, path).await,
        }
    }

    /// Resolves a `RemotePathCommitOptional` to a `RemotePath`.
    pub async fn resolve_path<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        kind: crate::retrieval::Kind,
        path: &objectiveai::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai::RemotePath>, ResponseError> {
        let remote = path.remote();
        let cache_key = path.clone();
        let router = self.clone();
        let ctx_clone = ctx.clone();
        let path_clone = path.clone();
        ctx.cached_remote_latest(cache_key, move || async move {
            router.dispatch_resolve_latest(remote, &ctx_clone, kind, &path_clone).await
        }).await
    }

    // ── Agent ──────────────────────────────────────────────────────

    /// Resolve an agent: inline converts directly, remote fetches and converts.
    pub async fn get_agent<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    ) -> Result<objectiveai::agent::AgentWithFallbacks, ResponseError> {
        match params {
            objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(base) => {
                let converted = base.convert().map_err(|e| bad_request(&e))?;
                Ok(objectiveai::agent::AgentWithFallbacks::Inline(converted))
            }
            objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::Remote(remote) => {
                let base = self.fetch_agent_base(ctx, &remote).await?
                    .ok_or_else(|| not_found("agent"))?;
                let converted = base.convert().map_err(|e| bad_request(&e))?;
                Ok(objectiveai::agent::AgentWithFallbacks::Remote(converted))
            }
        }
    }

    /// Fetch a raw `RemoteAgentBaseWithFallbacks` from a source, with per-request dedup caching.
    async fn fetch_agent_base<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai::agent::RemoteAgentBaseWithFallbacks>, ResponseError> {
        let Some(path) = self.resolve_path(ctx, crate::retrieval::Kind::Agents, params).await? else {
            return Ok(None);
        };
        let router = self.clone();
        let remote = path.remote();
        let path_clone = path.clone();
        let ctx_clone = ctx.clone();
        ctx.cached_agent(path, move || async move {
            router.dispatch_get_agent(remote, &ctx_clone, &path_clone).await
        }).await
    }

    /// API endpoint: fetch a remote agent, convert, wrap in response.
    pub async fn endpoint_get_agent<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai::RemotePathCommitOptional,
    ) -> Result<objectiveai::agent::response::GetAgentResponse, ResponseError> {
        let path = self.resolve_path(ctx, crate::retrieval::Kind::Agents, params).await?
            .ok_or_else(|| not_found("agent"))?;
        let result = self.get_agent(
            ctx,
            objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::Remote(params.clone()),
        ).await?;
        let inner = match result {
            objectiveai::agent::AgentWithFallbacks::Remote(r) => r,
            objectiveai::agent::AgentWithFallbacks::Inline(_) => unreachable!(),
        };
        Ok(objectiveai::agent::response::GetAgentResponse { path, inner })
    }

    // ── Swarm ─────────────────────────────────────────────────────

    /// Resolve a swarm: inline converts directly, remote fetches and converts.
    /// Remote agent references in the swarm's agents list are resolved automatically.
    pub async fn get_swarm<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional,
    ) -> Result<objectiveai::swarm::Swarm, ResponseError> {
        match params {
            objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(base) => {
                let converted = self.resolve_swarm_base(ctx, base).await?;
                Ok(objectiveai::swarm::Swarm::Inline(converted))
            }
            objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::Remote(remote) => {
                let base = self.fetch_swarm_base(ctx, &remote).await?
                    .ok_or_else(|| not_found("swarm"))?;
                let converted = self.resolve_swarm_base(ctx, base.inner).await?;
                Ok(objectiveai::swarm::Swarm::Remote(objectiveai::swarm::RemoteSwarm {
                    description: base.description,
                    inner: converted,
                }))
            }
        }
    }

    /// Resolve remote agent references in a swarm base and convert it.
    /// All remote agents are fetched concurrently.
    async fn resolve_swarm_base<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        base: objectiveai::swarm::InlineSwarmBase,
    ) -> Result<objectiveai::swarm::InlineSwarm, ResponseError> {
        // Collect unique remote agent paths to fetch.
        let mut unique_paths: indexmap::IndexMap<String, objectiveai::RemotePathCommitOptional> =
            indexmap::IndexMap::new();
        for agent_slot in &base.agents {
            if let objectiveai::agent::InlineAgentBaseWithFallbacksOrRemote::Remote(path) =
                &agent_slot.inner
            {
                let key = path.key();
                unique_paths
                    .entry(key)
                    .or_insert_with(|| path.clone().into());
            }
        }

        // Fetch all remote agents concurrently.
        if !unique_paths.is_empty() {
            let futs: Vec<_> = unique_paths
                .iter()
                .map(|(key, path)| {
                    let key = key.clone();
                    async move {
                        let agent_base = self.fetch_agent_base(ctx, path).await?
                            .ok_or_else(|| not_found("agent"))?;
                        Ok::<_, ResponseError>((key, agent_base))
                    }
                })
                .collect();
            let results = futures::future::try_join_all(futs).await?;
            let remote_agents: std::collections::HashMap<_, _> = results.into_iter().collect();
            base.convert(Some(&remote_agents)).map_err(|e| bad_request(&e))
        } else {
            base.convert(None).map_err(|e| bad_request(&e))
        }
    }

    /// Fetch a raw `RemoteSwarmBase` from a source, with per-request dedup caching.
    /// Falls back to swarm.json if profile.json is not found (for profile retrieval).
    async fn fetch_swarm_base<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai::swarm::RemoteSwarmBase>, ResponseError> {
        let Some(path) = self.resolve_path(ctx, crate::retrieval::Kind::Swarms, params).await? else {
            return Ok(None);
        };
        let router = self.clone();
        let remote = path.remote();
        let path_clone = path.clone();
        let ctx_clone = ctx.clone();
        ctx.cached_swarm(path, move || async move {
            router.dispatch_get_swarm(remote, &ctx_clone, &path_clone).await
        }).await
    }

    /// API endpoint: fetch a remote swarm, convert, wrap in response.
    pub async fn endpoint_get_swarm<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai::RemotePathCommitOptional,
    ) -> Result<objectiveai::swarm::response::GetSwarmResponse, ResponseError> {
        let path = self.resolve_path(ctx, crate::retrieval::Kind::Swarms, params).await?
            .ok_or_else(|| not_found("swarm"))?;
        let result = self.get_swarm(
            ctx,
            objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::Remote(params.clone()),
        ).await?;
        let inner = match result {
            objectiveai::swarm::Swarm::Remote(r) => r,
            objectiveai::swarm::Swarm::Inline(_) => unreachable!(),
        };
        Ok(objectiveai::swarm::response::GetSwarmResponse { path, inner })
    }

    // ── Function ──────────────────────────────────────────────────

    /// Resolve a function: inline returns directly, remote fetches with caching.
    pub async fn get_function<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: objectiveai::functions::FullInlineFunctionOrRemoteCommitOptional,
    ) -> Result<objectiveai::functions::FullFunction, ResponseError> {
        match params {
            objectiveai::functions::FullInlineFunctionOrRemoteCommitOptional::Inline(inline) => {
                Ok(objectiveai::functions::FullFunction::Inline(inline))
            }
            objectiveai::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(remote) => {
                let fetched = self.fetch_function(ctx, &remote).await?
                    .ok_or_else(|| not_found("function"))?;
                Ok(objectiveai::functions::FullFunction::Remote(fetched))
            }
        }
    }

    /// Fetch a raw `FullRemoteFunction` from a source, with per-request dedup caching.
    async fn fetch_function<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai::functions::FullRemoteFunction>, ResponseError> {
        let Some(path) = self.resolve_path(ctx, crate::retrieval::Kind::Functions, params).await? else {
            return Ok(None);
        };
        let router = self.clone();
        let remote = path.remote();
        let path_clone = path.clone();
        let ctx_clone = ctx.clone();
        ctx.cached_function(path, move || async move {
            router.dispatch_get_function(remote, &ctx_clone, &path_clone).await
        }).await
    }

    /// API endpoint: fetch a remote function, wrap in response.
    pub async fn endpoint_get_function<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai::RemotePathCommitOptional,
    ) -> Result<objectiveai::functions::response::GetFunctionResponse, ResponseError> {
        let path = self.resolve_path(ctx, crate::retrieval::Kind::Functions, params).await?
            .ok_or_else(|| not_found("function"))?;
        let result = self.get_function(
            ctx,
            objectiveai::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(params.clone()),
        ).await?;
        let inner = match result {
            objectiveai::functions::FullFunction::Remote(r) => r,
            objectiveai::functions::FullFunction::Inline(_) => unreachable!(),
        };
        Ok(objectiveai::functions::response::GetFunctionResponse { path, inner })
    }

    /// Fetches all child functions referenced by a function's tasks.
    /// Returns a HashMap keyed by the path's key string.
    pub async fn get_function_tasks<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        function: objectiveai::functions::FullFunction,
    ) -> Result<std::collections::HashMap<String, objectiveai::functions::FullRemoteFunction>, ResponseError> {
        let mut futs: Vec<(String, _)> = Vec::new();

        for path in function.remotes() {
            let key = path.key();
            let params = objectiveai::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(
                path.clone().into(),
            );
            let router = self.clone();
            let ctx = ctx.clone();
            futs.push((key, tokio::spawn(async move {
                router.get_function(&ctx, params).await
            })));
        }

        let mut children = std::collections::HashMap::new();
        for (key, handle) in futs {
            let full_fn = handle.await.expect("get_function_tasks panicked")?;
            match full_fn {
                objectiveai::functions::FullFunction::Remote(r) => {
                    children.insert(key, r);
                }
                objectiveai::functions::FullFunction::Inline(_) => {
                    unreachable!()
                }
            }
        }

        Ok(children)
    }

    // ── Profile ───────────────────────────────────────────────────

    /// Resolve a profile: inline returns directly, remote fetches with caching.
    pub async fn get_profile<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: objectiveai::functions::InlineProfileOrRemoteCommitOptional,
    ) -> Result<objectiveai::functions::Profile, ResponseError> {
        match params {
            objectiveai::functions::InlineProfileOrRemoteCommitOptional::Inline(inline) => {
                Ok(objectiveai::functions::Profile::Inline(inline))
            }
            objectiveai::functions::InlineProfileOrRemoteCommitOptional::Remote(remote) => {
                let fetched = self.fetch_profile(ctx, &remote).await?
                    .ok_or_else(|| not_found("profile"))?;
                Ok(objectiveai::functions::Profile::Remote(fetched))
            }
        }
    }

    /// Fetch a raw `RemoteProfile` from a source, with per-request dedup caching.
    /// Falls back to swarm.json if profile.json is not found.
    async fn fetch_profile<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai::functions::RemoteProfile>, ResponseError> {
        let Some(path) = self.resolve_path(ctx, crate::retrieval::Kind::Profiles, params).await? else {
            return Ok(None);
        };
        let router = self.clone();
        let remote = path.remote();
        let path_clone = path.clone();
        let ctx_clone = ctx.clone();
        ctx.cached_profile(path, move || async move {
            // Try profile.json first
            let result = router.dispatch_get_profile(remote, &ctx_clone, &path_clone).await;
            match &result {
                Ok(None) => {
                    // Fallback: try swarm.json (a swarm definition is a valid Auto profile)
                    match router.dispatch_get_swarm(remote, &ctx_clone, &path_clone).await {
                        Ok(Some(swarm)) => Ok(Some(
                            objectiveai::functions::RemoteProfile::Auto(swarm),
                        )),
                        Ok(None) => Ok(None),
                        Err(e) => Err(e),
                    }
                }
                _ => result,
            }
        }).await
    }

    // ── Function Invention State ────────────────────────────────────

    /// Resolves an invention state: if inline, returns the ParamsState directly.
    /// If remote, fetches the constituent files and deserializes into ParamsState.
    /// Returns None if the remote path doesn't exist or all files are missing.
    pub async fn get_function_invention_state<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: objectiveai::functions::inventions::ParamsStateOrRemoteCommitOptional,
    ) -> Result<Option<objectiveai::functions::inventions::ParamsState>, ResponseError> {
        let remote_path = match params {
            objectiveai::functions::inventions::ParamsStateOrRemoteCommitOptional::Inline(state) => {
                return Ok(Some(state));
            }
            objectiveai::functions::inventions::ParamsStateOrRemoteCommitOptional::Remote(remote) => remote,
        };

        let Some(path) = self.resolve_path(ctx, crate::retrieval::Kind::Functions, &remote_path).await? else {
            return Ok(None);
        };
        let remote = path.remote();

        // Fetch all state files concurrently
        let filenames = objectiveai::functions::inventions::ParamsState::filenames();
        let futs: Vec<_> = filenames.iter().map(|&filename| {
            let path = path.clone();
            let ctx = ctx.clone();
            async move {
                let content = match remote {
                    Remote::Github => self.github.get_function_invention_state_file(&ctx, &path, filename).await,
                    Remote::Filesystem => self.filesystem.get_function_invention_state_file(&ctx, &path, filename).await,
                    Remote::Mock => self.mock.get_function_invention_state_file(&ctx, &path, filename).await,
                }?;
                Ok::<_, ResponseError>((filename, content))
            }
        }).collect();

        let results = futures::future::try_join_all(futs).await?;

        // If all files are None, no state exists
        if results.iter().all(|(_, content)| content.is_none()) {
            return Ok(None);
        }

        // Build HashMap from the results (only include files that exist)
        let map: std::collections::HashMap<&'static str, String> = results
            .into_iter()
            .filter_map(|(filename, content)| content.map(|c| (filename, c)))
            .collect();

        match objectiveai::functions::inventions::ParamsState::deserialize_from_files(map) {
            Ok(state) => Ok(state),
            Err(e) => Err(ResponseError {
                code: 500,
                message: serde_json::json!({ "error": format!("failed to deserialize invention state: {e}") }),
            }),
        }
    }

    /// API endpoint: fetch a remote function invention state, wrap in response.
    pub async fn endpoint_get_function_invention_state<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai::RemotePathCommitOptional,
    ) -> Result<objectiveai::functions::inventions::state::response::GetFunctionInventionStateResponse, ResponseError> {
        let path = self.resolve_path(ctx, crate::retrieval::Kind::Functions, params).await?
            .ok_or_else(|| not_found("function invention state"))?;
        let state = self.get_function_invention_state(
            ctx,
            objectiveai::functions::inventions::ParamsStateOrRemoteCommitOptional::Remote(params.clone()),
        ).await?
            .ok_or_else(|| not_found("function invention state"))?;
        Ok(objectiveai::functions::inventions::state::response::GetFunctionInventionStateResponse { path, inner: state })
    }

    /// API endpoint: fetch a remote profile, wrap in response.
    pub async fn endpoint_get_profile<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai::RemotePathCommitOptional,
    ) -> Result<objectiveai::functions::profiles::response::GetProfileResponse, ResponseError> {
        let path = self.resolve_path(ctx, crate::retrieval::Kind::Profiles, params).await?
            .ok_or_else(|| not_found("profile"))?;
        let result = self.get_profile(
            ctx,
            objectiveai::functions::InlineProfileOrRemoteCommitOptional::Remote(params.clone()),
        ).await?;
        let inner = match result {
            objectiveai::functions::Profile::Remote(r) => r,
            objectiveai::functions::Profile::Inline(_) => unreachable!(),
        };
        Ok(objectiveai::functions::profiles::response::GetProfileResponse { path, inner })
    }

    // ── Prompt ─────────────────────────────────────────────────────

    /// Resolve a prompt: inline returns directly, remote fetches with caching.
    pub async fn get_prompt<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: objectiveai::functions::inventions::prompts::InlinePromptOrRemoteCommitOptional,
    ) -> Result<objectiveai::functions::inventions::prompts::Prompt, ResponseError> {
        match params {
            objectiveai::functions::inventions::prompts::InlinePromptOrRemoteCommitOptional::Inline(inline) => {
                Ok(objectiveai::functions::inventions::prompts::Prompt::Inline(inline))
            }
            objectiveai::functions::inventions::prompts::InlinePromptOrRemoteCommitOptional::Remote(remote) => {
                let fetched = self.fetch_prompt(ctx, &remote).await?
                    .ok_or_else(|| not_found("prompt"))?;
                Ok(objectiveai::functions::inventions::prompts::Prompt::Remote(fetched))
            }
        }
    }

    /// Fetch a raw `RemotePrompt` from a source.
    async fn fetch_prompt<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai::functions::inventions::prompts::RemotePrompt>, ResponseError> {
        let Some(path) = self.resolve_path(ctx, crate::retrieval::Kind::Prompts, params).await? else {
            return Ok(None);
        };
        let remote = path.remote();
        self.dispatch_get_prompt(remote, ctx, &path).await
    }

    /// API endpoint: fetch a remote prompt, wrap in response.
    pub async fn endpoint_get_prompt<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT, PC>,
        params: &objectiveai::RemotePathCommitOptional,
    ) -> Result<objectiveai::functions::inventions::prompts::response::GetPromptResponse, ResponseError> {
        let path = self.resolve_path(ctx, crate::retrieval::Kind::Prompts, params).await?
            .ok_or_else(|| not_found("prompt"))?;
        let result = self.get_prompt(
            ctx,
            objectiveai::functions::inventions::prompts::InlinePromptOrRemoteCommitOptional::Remote(params.clone()),
        ).await?;
        let inner = match result {
            objectiveai::functions::inventions::prompts::Prompt::Remote(r) => r,
            objectiveai::functions::inventions::prompts::Prompt::Inline(_) => unreachable!(),
        };
        Ok(objectiveai::functions::inventions::prompts::response::GetPromptResponse { path, inner })
    }
}

fn not_found(kind: &str) -> ResponseError {
    ResponseError {
        code: 404,
        message: serde_json::json!({ "error": format!("{} not found", kind) }),
    }
}

fn bad_request(msg: &str) -> ResponseError {
    ResponseError {
        code: 400,
        message: serde_json::json!({ "error": msg }),
    }
}
