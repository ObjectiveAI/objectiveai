//! `Client` remote resolver — resolves over the websocket reverse-channel.
//!
//! A `Client` remote means "ask the connected client to resolve this".
//! This implementor forwards each retrieval to the client over the
//! reverse-attach websocket (the same machinery behind
//! `client_objectiveai_mcp`) and awaits the reply. If the request isn't
//! over a websocket (no reverse-attach on the ctx), resolution fails —
//! mirroring how client objectiveai MCP tools fail without a websocket.

use crate::ctx;
use objectiveai_sdk::client_objectiveai_mcp::{retrieve, server_request, server_response};
use objectiveai_sdk::error::ResponseError;

/// Stateless resolver — everything it needs comes from the `ctx`'s
/// reverse-attach handle at call time.
pub struct ClientClient;

impl ClientClient {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClientClient {
    fn default() -> Self {
        Self::new()
    }
}

fn reverse_error(message: impl Into<String>) -> ResponseError {
    ResponseError {
        code: 400,
        message: serde_json::json!({ "error": message.into() }),
    }
}

fn protocol_kind(kind: crate::retrieval::Kind) -> retrieve::Kind {
    match kind {
        crate::retrieval::Kind::Agents => retrieve::Kind::Agents,
        crate::retrieval::Kind::Swarms => retrieve::Kind::Swarms,
        crate::retrieval::Kind::Functions => retrieve::Kind::Functions,
        crate::retrieval::Kind::Profiles => retrieve::Kind::Profiles,
    }
}

/// Send a resolution request down the reverse channel and await the
/// typed reply. Fails when there is no websocket reverse-attach.
async fn send_retrieve<CTXEXT>(
    ctx: &ctx::Context<CTXEXT>,
    request: retrieve::Request,
) -> Result<retrieve::Response, ResponseError>
{
    let handle = ctx.reverse_attach().ok_or_else(|| {
        reverse_error("`client` remote requires a websocket connection")
    })?;
    let rc = handle.channel();
    let request = server_request::Request {
        id: uuid::Uuid::new_v4().to_string(),
        headers: indexmap::IndexMap::new(),
        payload: server_request::Payload::Retrieve(request),
    };
    let rx = crate::objectiveai_mcp::send_server_request(&rc.sink, &rc.pending, request)
        .await
        .map_err(|()| reverse_error("reverse channel closed"))?;
    let response = match tokio::time::timeout(handle.reverse_channel_timeout(), rx).await {
        Ok(Ok(response)) => response,
        Ok(Err(_)) => {
            return Err(reverse_error("reverse channel dropped before reply"));
        }
        Err(_) => return Err(reverse_error("reverse channel timed out")),
    };
    match response.payload {
        server_response::Payload::Retrieve(server_response::JsonRpcResult::Ok { result }) => {
            Ok(result)
        }
        server_response::Payload::Retrieve(server_response::JsonRpcResult::Err {
            code,
            message,
            ..
        }) => Err(reverse_error(format!("client returned error {code}: {message}"))),
        other => Err(reverse_error(format!("client returned wrong reply variant: {other:?}"))),
    }
}

#[async_trait::async_trait]
impl<CTXEXT> super::super::Client<CTXEXT> for ClientClient
where
    CTXEXT: Send + Sync + 'static,
{
    async fn get_agent(
        &self,
        ctx: &ctx::Context<CTXEXT>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks>, ResponseError> {
        match send_retrieve(ctx, retrieve::Request::GetAgent { path: path.clone() }).await? {
            retrieve::Response::GetAgent { agent } => Ok(agent),
            other => Err(reverse_error(format!("client returned wrong reply variant: {other:?}"))),
        }
    }

    async fn get_swarm(
        &self,
        ctx: &ctx::Context<CTXEXT>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::swarm::RemoteSwarmBase>, ResponseError> {
        match send_retrieve(ctx, retrieve::Request::GetSwarm { path: path.clone() }).await? {
            retrieve::Response::GetSwarm { swarm } => Ok(swarm),
            other => Err(reverse_error(format!("client returned wrong reply variant: {other:?}"))),
        }
    }

    async fn get_function(
        &self,
        ctx: &ctx::Context<CTXEXT>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::functions::FullRemoteFunction>, ResponseError> {
        match send_retrieve(ctx, retrieve::Request::GetFunction { path: path.clone() }).await? {
            retrieve::Response::GetFunction { function } => Ok(function),
            other => Err(reverse_error(format!("client returned wrong reply variant: {other:?}"))),
        }
    }

    async fn get_profile(
        &self,
        ctx: &ctx::Context<CTXEXT>,
        path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::functions::RemoteProfile>, ResponseError> {
        match send_retrieve(ctx, retrieve::Request::GetProfile { path: path.clone() }).await? {
            retrieve::Response::GetProfile { profile } => Ok(profile),
            other => Err(reverse_error(format!("client returned wrong reply variant: {other:?}"))),
        }
    }

    async fn resolve_latest(
        &self,
        ctx: &ctx::Context<CTXEXT>,
        kind: crate::retrieval::Kind,
        path: &objectiveai_sdk::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai_sdk::RemotePath>, ResponseError> {
        let request = retrieve::Request::ResolveLatest {
            kind: protocol_kind(kind),
            path: path.clone(),
        };
        match send_retrieve(ctx, request).await? {
            retrieve::Response::ResolveLatest { path } => Ok(path),
            other => Err(reverse_error(format!("client returned wrong reply variant: {other:?}"))),
        }
    }
}
