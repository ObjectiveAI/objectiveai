//! Local resolution of agents / swarms / functions / profiles.
//!
//! These commands resolve content directly — never via the ObjectiveAI
//! API. Allowed remotes are **GitHub** (via the SDK `http::github`
//! methods, authenticated with the client's `x_github_authorization`)
//! and the local **filesystem**; `mock` is rejected.
//!
//! - `get_*` resolves a single definition and converts it to the full
//!   form (computed IDs), matching the API's `Get*Response` shape.
//!   Swarm conversion first resolves the swarm's embedded remote agents.
//! - `list` enumerates the local filesystem only and returns paths.

use std::collections::HashMap;
use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::agent::{
    InlineAgentBaseWithFallbacksOrRemote, RemoteAgentBaseWithFallbacks,
};
use objectiveai_sdk::cli::command::agents::get::Response as GetAgentResponse;
use objectiveai_sdk::cli::command::functions::get::Response as GetFunctionResponse;
use objectiveai_sdk::cli::command::functions::profiles::get::Response as GetProfileResponse;
use objectiveai_sdk::cli::command::swarms::get::Response as GetSwarmResponse;
use objectiveai_sdk::functions::{FullRemoteFunction, RemoteProfile};
use objectiveai_sdk::swarm::RemoteSwarmBase;
use objectiveai_sdk::{HttpClient, RemotePath, RemotePathCommitOptional};

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::publish::Kind;

// ── list (filesystem only) ─────────────────────────────────────────────

/// Streams locally-published repositories of `kind`. GitHub has no list.
pub async fn list(
    ctx: &Context,
    kind: Kind,
) -> Pin<Box<dyn Stream<Item = RemotePath> + Send>> {
    ctx.filesystem.list(kind).await
}

// ── get ────────────────────────────────────────────────────────────────

pub async fn get_agent(
    ctx: &Context,
    path: RemotePathCommitOptional,
) -> Result<GetAgentResponse, Error> {
    let http = ctx.github_http_client().await?;
    let (base, path) = fetch_agent_base(ctx, &http, &path).await?;
    let inner = base.convert().map_err(Error::AgentConvert)?;
    Ok(GetAgentResponse { path, inner })
}

pub async fn get_swarm(
    ctx: &Context,
    path: RemotePathCommitOptional,
) -> Result<GetSwarmResponse, Error> {
    let http = ctx.github_http_client().await?;
    let (base, path) = fetch_swarm_base(ctx, &http, &path).await?;
    let remote_agents = resolve_swarm_agents(ctx, &http, &base).await?;
    let inner =
        base.convert(remote_agents.as_ref()).map_err(Error::SwarmConvert)?;
    Ok(GetSwarmResponse { path, inner })
}

pub async fn get_function(
    ctx: &Context,
    path: RemotePathCommitOptional,
) -> Result<GetFunctionResponse, Error> {
    let http = ctx.github_http_client().await?;
    let (inner, path) = fetch_function(ctx, &http, &path).await?;
    Ok(GetFunctionResponse { path, inner })
}

pub async fn get_profile(
    ctx: &Context,
    path: RemotePathCommitOptional,
) -> Result<GetProfileResponse, Error> {
    let http = ctx.github_http_client().await?;
    let (inner, path) = fetch_profile(ctx, &http, &path).await?;
    Ok(GetProfileResponse { path, inner })
}

// ── base fetchers (github / filesystem; mock rejected) ──────────────────

async fn fetch_agent_base(
    ctx: &Context,
    http: &HttpClient,
    path: &RemotePathCommitOptional,
) -> Result<(RemoteAgentBaseWithFallbacks, RemotePath), Error> {
    match path {
        RemotePathCommitOptional::Github { owner, repository, commit } => {
            let commit =
                resolve_github_commit(http, owner, repository, commit.as_deref())
                    .await?;
            let base = http
                .github_get_agent(owner, repository, Some(&commit))
                .await?
                .ok_or_else(|| not_found("agent", owner, repository))?;
            Ok((base, github_path(owner, repository, commit)))
        }
        RemotePathCommitOptional::Filesystem { owner, repository, commit } => {
            let (base, commit) = ctx
                .filesystem
                .read_json::<RemoteAgentBaseWithFallbacks>(
                    Kind::Agents,
                    owner,
                    repository,
                    commit.as_deref(),
                )
                .await?
                .ok_or_else(|| not_found("agent", owner, repository))?;
            Ok((base, fs_path(owner, repository, commit)))
        }
        RemotePathCommitOptional::Mock { .. } => {
            Err(Error::RemoteNotSupported("mock"))
        }
    }
}

async fn fetch_swarm_base(
    ctx: &Context,
    http: &HttpClient,
    path: &RemotePathCommitOptional,
) -> Result<(RemoteSwarmBase, RemotePath), Error> {
    match path {
        RemotePathCommitOptional::Github { owner, repository, commit } => {
            let commit =
                resolve_github_commit(http, owner, repository, commit.as_deref())
                    .await?;
            let base = http
                .github_get_swarm(owner, repository, Some(&commit))
                .await?
                .ok_or_else(|| not_found("swarm", owner, repository))?;
            Ok((base, github_path(owner, repository, commit)))
        }
        RemotePathCommitOptional::Filesystem { owner, repository, commit } => {
            let (base, commit) = ctx
                .filesystem
                .read_json::<RemoteSwarmBase>(
                    Kind::Swarms,
                    owner,
                    repository,
                    commit.as_deref(),
                )
                .await?
                .ok_or_else(|| not_found("swarm", owner, repository))?;
            Ok((base, fs_path(owner, repository, commit)))
        }
        RemotePathCommitOptional::Mock { .. } => {
            Err(Error::RemoteNotSupported("mock"))
        }
    }
}

async fn fetch_function(
    ctx: &Context,
    http: &HttpClient,
    path: &RemotePathCommitOptional,
) -> Result<(FullRemoteFunction, RemotePath), Error> {
    match path {
        RemotePathCommitOptional::Github { owner, repository, commit } => {
            let commit =
                resolve_github_commit(http, owner, repository, commit.as_deref())
                    .await?;
            let inner = http
                .github_get_function(owner, repository, Some(&commit))
                .await?
                .ok_or_else(|| not_found("function", owner, repository))?;
            Ok((inner, github_path(owner, repository, commit)))
        }
        RemotePathCommitOptional::Filesystem { owner, repository, commit } => {
            let (inner, commit) = ctx
                .filesystem
                .read_json::<FullRemoteFunction>(
                    Kind::Functions,
                    owner,
                    repository,
                    commit.as_deref(),
                )
                .await?
                .ok_or_else(|| not_found("function", owner, repository))?;
            Ok((inner, fs_path(owner, repository, commit)))
        }
        RemotePathCommitOptional::Mock { .. } => {
            Err(Error::RemoteNotSupported("mock"))
        }
    }
}

async fn fetch_profile(
    ctx: &Context,
    http: &HttpClient,
    path: &RemotePathCommitOptional,
) -> Result<(RemoteProfile, RemotePath), Error> {
    match path {
        RemotePathCommitOptional::Github { owner, repository, commit } => {
            let commit =
                resolve_github_commit(http, owner, repository, commit.as_deref())
                    .await?;
            let inner = http
                .github_get_profile(owner, repository, Some(&commit))
                .await?
                .ok_or_else(|| not_found("profile", owner, repository))?;
            Ok((inner, github_path(owner, repository, commit)))
        }
        RemotePathCommitOptional::Filesystem { owner, repository, commit } => {
            let (inner, commit) = ctx
                .filesystem
                .read_json::<RemoteProfile>(
                    Kind::Profiles,
                    owner,
                    repository,
                    commit.as_deref(),
                )
                .await?
                .ok_or_else(|| not_found("profile", owner, repository))?;
            Ok((inner, fs_path(owner, repository, commit)))
        }
        RemotePathCommitOptional::Mock { .. } => {
            Err(Error::RemoteNotSupported("mock"))
        }
    }
}

/// Resolves a swarm's embedded remote agent references into the map that
/// [`RemoteSwarmBase::convert`] needs (mirrors the API's
/// `resolve_swarm_base`). Returns `None` when there are no remote agents.
async fn resolve_swarm_agents(
    ctx: &Context,
    http: &HttpClient,
    base: &RemoteSwarmBase,
) -> Result<
    Option<HashMap<String, (RemoteAgentBaseWithFallbacks, RemotePath)>>,
    Error,
> {
    let mut map: HashMap<String, (RemoteAgentBaseWithFallbacks, RemotePath)> =
        HashMap::new();
    for slot in &base.inner.agents {
        if let InlineAgentBaseWithFallbacksOrRemote::Remote(remote) =
            &slot.inner
        {
            let key = remote.key();
            if map.contains_key(&key) {
                continue;
            }
            let agent =
                fetch_agent_base(ctx, http, &remote.clone().into()).await?;
            map.insert(key, agent);
        }
    }
    Ok(if map.is_empty() { None } else { Some(map) })
}

// ── helpers ─────────────────────────────────────────────────────────────

async fn resolve_github_commit(
    http: &HttpClient,
    owner: &str,
    repository: &str,
    commit: Option<&str>,
) -> Result<String, Error> {
    match commit {
        Some(c) => Ok(c.to_string()),
        None => http
            .github_resolve_commit(owner, repository)
            .await?
            .ok_or_else(|| Error::NotFound(format!("{owner}/{repository}"))),
    }
}

fn github_path(owner: &str, repository: &str, commit: String) -> RemotePath {
    RemotePath::Github {
        owner: owner.to_string(),
        repository: repository.to_string(),
        commit,
    }
}

fn fs_path(owner: &str, repository: &str, commit: String) -> RemotePath {
    RemotePath::Filesystem {
        owner: owner.to_string(),
        repository: repository.to_string(),
        commit,
    }
}

fn not_found(kind: &str, owner: &str, repository: &str) -> Error {
    Error::NotFound(format!("{kind} {owner}/{repository}"))
}
