//! Retrieve router — dispatches by `Remote`, resolves commits, and caches per request.

use crate::ctx;
use objectiveai_sdk::error::ResponseError;
use objectiveai_sdk::Remote;
use std::sync::Arc;

/// Routes fetch operations by `Remote` to GitHub/Client/Mock,
/// with per-request deduplication caching via context caches.
///
/// Main methods accept `CommitOptional` enums (inline or remote ref).
/// If inline, converts directly. If remote, resolves commit, fetches
/// from source, converts, and returns the union type.
pub struct Router<G, C, M, CTXEXT> {
    pub github: Arc<G>,
    pub client: Arc<C>,
    pub mock: Arc<M>,
    _ctxext: std::marker::PhantomData<CTXEXT>,
}

impl<G, C, M, CTXEXT> Router<G, C, M, CTXEXT> {
    pub fn new(github: Arc<G>, client: Arc<C>, mock: Arc<M>) -> Self {
        Self { github, client, mock, _ctxext: std::marker::PhantomData }
    }
}


impl<G, C, M, CTXEXT> Router<G, C, M, CTXEXT>
where
    G: super::Client<CTXEXT>,
    C: super::Client<CTXEXT>,
    M: super::Client<CTXEXT>,
    CTXEXT: Send + Sync + 'static,
{
    async fn dispatch_resolve_latest(
        &self,
        remote: Remote,
        ctx: &ctx::Context<CTXEXT>,
        kind: crate::retrieval::Kind,
        path: &objectiveai_sdk::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai_sdk::RemotePath>, ResponseError> {
        match remote {
            Remote::Github => self.github.resolve_latest(ctx, kind, path).await,
            Remote::Client => self.client.resolve_latest(ctx, kind, path).await,
            Remote::Mock => self.mock.resolve_latest(ctx, kind, path).await,
        }
    }

    async fn dispatch_get_agent(
        &self,
        remote: Remote,
        ctx: &ctx::Context<CTXEXT>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks>, ResponseError> {
        match remote {
            Remote::Github => self.github.get_agent(ctx, path).await,
            Remote::Client => self.client.get_agent(ctx, path).await,
            Remote::Mock => self.mock.get_agent(ctx, path).await,
        }
    }

    async fn dispatch_get_swarm(
        &self,
        remote: Remote,
        ctx: &ctx::Context<CTXEXT>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::swarm::RemoteSwarmBase>, ResponseError> {
        match remote {
            Remote::Github => self.github.get_swarm(ctx, path).await,
            Remote::Client => self.client.get_swarm(ctx, path).await,
            Remote::Mock => self.mock.get_swarm(ctx, path).await,
        }
    }

    async fn dispatch_get_function(
        &self,
        remote: Remote,
        ctx: &ctx::Context<CTXEXT>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::functions::FullRemoteFunction>, ResponseError> {
        match remote {
            Remote::Github => self.github.get_function(ctx, path).await,
            Remote::Client => self.client.get_function(ctx, path).await,
            Remote::Mock => self.mock.get_function(ctx, path).await,
        }
    }

    async fn dispatch_get_profile(
        &self,
        remote: Remote,
        ctx: &ctx::Context<CTXEXT>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::functions::RemoteProfile>, ResponseError> {
        match remote {
            Remote::Github => self.github.get_profile(ctx, path).await,
            Remote::Client => self.client.get_profile(ctx, path).await,
            Remote::Mock => self.mock.get_profile(ctx, path).await,
        }
    }

    /// Resolves a `RemotePathCommitOptional` to a `RemotePath`.
    pub async fn resolve_path(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT>,
        kind: crate::retrieval::Kind,
        path: &objectiveai_sdk::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai_sdk::RemotePath>, ResponseError> {
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
    ///
    /// The returned `Option<RemotePath>` is `Some(path)` when the request
    /// supplied a remote (in which case the resolved-latest path is
    /// surfaced for stamping on response chunks), and `None` for inline
    /// requests.
    pub async fn get_agent(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT>,
        params: objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    ) -> Result<(objectiveai_sdk::agent::AgentWithFallbacks, Option<objectiveai_sdk::RemotePath>), ResponseError> {
        match params {
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(base) => {
                let converted = base.convert().map_err(|e| bad_request(&e))?;
                Ok((objectiveai_sdk::agent::AgentWithFallbacks::Inline(converted), None))
            }
            objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::Remote(remote) => {
                let (base, path) = self.fetch_agent_base(ctx, &remote).await?
                    .ok_or_else(|| not_found("agent"))?;
                let converted = base.convert().map_err(|e| bad_request(&e))?;
                Ok((objectiveai_sdk::agent::AgentWithFallbacks::Remote(converted), Some(path)))
            }
        }
    }

    /// Fetch a raw `RemoteAgentBaseWithFallbacks` from a source, with per-request dedup caching.
    ///
    /// Returns `(base, resolved_path)` so callers can stamp the path
    /// onto downstream response shapes without re-resolving.
    async fn fetch_agent_base(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT>,
        params: &objectiveai_sdk::RemotePathCommitOptional,
    ) -> Result<Option<(objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks, objectiveai_sdk::RemotePath)>, ResponseError> {
        let Some(path) = self.resolve_path(ctx, crate::retrieval::Kind::Agents, params).await? else {
            return Ok(None);
        };
        let router = self.clone();
        let remote = path.remote();
        let path_clone = path.clone();
        let ctx_clone = ctx.clone();
        let base = ctx.cached_agent(path.clone(), move || async move {
            router.dispatch_get_agent(remote, &ctx_clone, &path_clone).await
        }).await?;
        Ok(base.map(|b| (b, path)))
    }

    // ── Swarm ─────────────────────────────────────────────────────

    /// Resolve a swarm: inline converts directly, remote fetches and converts.
    /// Remote agent references in the swarm's agents list are resolved automatically.
    pub async fn get_swarm(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT>,
        params: objectiveai_sdk::swarm::InlineSwarmBaseOrRemoteCommitOptional,
    ) -> Result<objectiveai_sdk::swarm::Swarm, ResponseError> {
        match params {
            objectiveai_sdk::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(base) => {
                let converted = self.resolve_swarm_base(ctx, base).await?;
                Ok(objectiveai_sdk::swarm::Swarm::Inline(converted))
            }
            objectiveai_sdk::swarm::InlineSwarmBaseOrRemoteCommitOptional::Remote(remote) => {
                let base = self.fetch_swarm_base(ctx, &remote).await?
                    .ok_or_else(|| not_found("swarm"))?;
                let converted = self.resolve_swarm_base(ctx, base.inner).await?;
                Ok(objectiveai_sdk::swarm::Swarm::Remote(objectiveai_sdk::swarm::RemoteSwarm {
                    description: base.description,
                    inner: converted,
                }))
            }
        }
    }

    /// Resolve remote agent references in a swarm base and convert it.
    /// All remote agents are fetched concurrently.
    async fn resolve_swarm_base(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT>,
        base: objectiveai_sdk::swarm::InlineSwarmBase,
    ) -> Result<objectiveai_sdk::swarm::InlineSwarm, ResponseError> {
        // Collect unique remote agent paths to fetch.
        let mut unique_paths: indexmap::IndexMap<String, objectiveai_sdk::RemotePathCommitOptional> =
            indexmap::IndexMap::new();
        for agent_slot in &base.agents {
            if let objectiveai_sdk::agent::InlineAgentBaseWithFallbacksOrRemote::Remote(path) =
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
    async fn fetch_swarm_base(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT>,
        params: &objectiveai_sdk::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai_sdk::swarm::RemoteSwarmBase>, ResponseError> {
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

    // ── Function ──────────────────────────────────────────────────

    /// Resolve a function: inline returns directly, remote fetches with caching.
    pub async fn get_function(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT>,
        params: objectiveai_sdk::functions::FullInlineFunctionOrRemoteCommitOptional,
    ) -> Result<objectiveai_sdk::functions::FullFunction, ResponseError> {
        match params {
            objectiveai_sdk::functions::FullInlineFunctionOrRemoteCommitOptional::Inline(inline) => {
                Ok(objectiveai_sdk::functions::FullFunction::Inline(inline))
            }
            objectiveai_sdk::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(remote) => {
                let fetched = self.fetch_function(ctx, &remote).await?
                    .ok_or_else(|| not_found("function"))?;
                Ok(objectiveai_sdk::functions::FullFunction::Remote(fetched))
            }
        }
    }

    /// Fetch a raw `FullRemoteFunction` from a source, with per-request dedup caching.
    async fn fetch_function(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT>,
        params: &objectiveai_sdk::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai_sdk::functions::FullRemoteFunction>, ResponseError> {
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

    /// Fetches all child functions referenced by a function's tasks.
    /// Returns a HashMap keyed by the path's key string.
    pub async fn get_function_tasks(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT>,
        function: objectiveai_sdk::functions::FullFunction,
    ) -> Result<std::collections::HashMap<String, objectiveai_sdk::functions::FullRemoteFunction>, ResponseError> {
        let mut futs: Vec<(String, _)> = Vec::new();

        for path in function.remotes() {
            let key = path.key();
            let params = objectiveai_sdk::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(
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
                objectiveai_sdk::functions::FullFunction::Remote(r) => {
                    children.insert(key, r);
                }
                objectiveai_sdk::functions::FullFunction::Inline(_) => {
                    unreachable!()
                }
            }
        }

        Ok(children)
    }

    /// Fetch a remote function and its resolved commit path (execution helper).
    pub async fn get_remote_function(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT>,
        params: &objectiveai_sdk::RemotePathCommitOptional,
    ) -> Result<(objectiveai_sdk::functions::FullRemoteFunction, objectiveai_sdk::RemotePath), ResponseError> {
        let path = self.resolve_path(ctx, crate::retrieval::Kind::Functions, params).await?
            .ok_or_else(|| not_found("function"))?;
        let result = self.get_function(
            ctx,
            objectiveai_sdk::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(params.clone()),
        ).await?;
        let inner = match result {
            objectiveai_sdk::functions::FullFunction::Remote(r) => r,
            objectiveai_sdk::functions::FullFunction::Inline(_) => unreachable!(),
        };
        Ok((inner, path))
    }

    // ── Profile ───────────────────────────────────────────────────

    /// Resolve a profile: inline returns directly, remote fetches with caching.
    pub async fn get_profile(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT>,
        params: objectiveai_sdk::functions::InlineProfileOrRemoteCommitOptional,
    ) -> Result<objectiveai_sdk::functions::Profile, ResponseError> {
        match params {
            objectiveai_sdk::functions::InlineProfileOrRemoteCommitOptional::Inline(inline) => {
                Ok(objectiveai_sdk::functions::Profile::Inline(inline))
            }
            objectiveai_sdk::functions::InlineProfileOrRemoteCommitOptional::Remote(remote) => {
                let fetched = self.fetch_profile(ctx, &remote).await?
                    .ok_or_else(|| not_found("profile"))?;
                Ok(objectiveai_sdk::functions::Profile::Remote(fetched))
            }
        }
    }

    /// Fetch a raw `RemoteProfile` from a source, with per-request dedup caching.
    /// Falls back to swarm.json if profile.json is not found.
    async fn fetch_profile(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT>,
        params: &objectiveai_sdk::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai_sdk::functions::RemoteProfile>, ResponseError> {
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
                            objectiveai_sdk::functions::RemoteProfile::Auto(swarm),
                        )),
                        Ok(None) => Ok(None),
                        Err(e) => Err(e),
                    }
                }
                _ => result,
            }
        }).await
    }

    /// Fetch a remote profile and its resolved commit path (execution helper).
    pub async fn get_remote_profile(
        self: &Arc<Self>,
        ctx: &ctx::Context<CTXEXT>,
        params: &objectiveai_sdk::RemotePathCommitOptional,
    ) -> Result<(objectiveai_sdk::functions::RemoteProfile, objectiveai_sdk::RemotePath), ResponseError> {
        let path = self.resolve_path(ctx, crate::retrieval::Kind::Profiles, params).await?
            .ok_or_else(|| not_found("profile"))?;
        let result = self.get_profile(
            ctx,
            objectiveai_sdk::functions::InlineProfileOrRemoteCommitOptional::Remote(params.clone()),
        ).await?;
        let inner = match result {
            objectiveai_sdk::functions::Profile::Remote(r) => r,
            objectiveai_sdk::functions::Profile::Inline(_) => unreachable!(),
        };
        Ok((inner, path))
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
