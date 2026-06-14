//! Reverse-channel resolution messages — the API asks the connected
//! client to resolve a `Client` remote (agent / swarm / function /
//! profile) from the client's own local storage and reply over the
//! websocket. Non-MCP, mirrors the `ReadMessageQueue` request shape.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::agent::RemoteAgentBaseWithFallbacks;
use crate::functions::{FullRemoteFunction, RemoteProfile};
use crate::swarm::RemoteSwarmBase;
use crate::{RemotePath, RemotePathCommitOptional};

/// Which content kind a [`Request::ResolveLatest`] targets — picks the
/// `<kind>/<owner>/<repository>` directory on the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.retrieve.Kind")]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Agents,
    Swarms,
    Functions,
    Profiles,
}

/// A resolution request the API forwards down the reverse channel.
/// Mirrors the API `retrieve::Client` trait: `Get*` fetch a base
/// definition at an exact [`RemotePath`]; `ResolveLatest` resolves a
/// [`RemotePathCommitOptional`] to a full [`RemotePath`] (filling in
/// the commit).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.retrieve.Request")]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    #[schemars(title = "GetAgent")]
    GetAgent { path: RemotePath },
    #[schemars(title = "GetSwarm")]
    GetSwarm { path: RemotePath },
    #[schemars(title = "GetFunction")]
    GetFunction { path: RemotePath },
    #[schemars(title = "GetProfile")]
    GetProfile { path: RemotePath },
    #[schemars(title = "ResolveLatest")]
    ResolveLatest {
        kind: Kind,
        path: RemotePathCommitOptional,
    },
}

/// The client's reply payload for a [`Request`]. The variant matches
/// the request's `op`; `None` means not found.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.retrieve.Response")]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Response {
    #[schemars(title = "GetAgent")]
    GetAgent {
        agent: Option<RemoteAgentBaseWithFallbacks>,
    },
    #[schemars(title = "GetSwarm")]
    GetSwarm { swarm: Option<RemoteSwarmBase> },
    #[schemars(title = "GetFunction")]
    GetFunction {
        function: Option<FullRemoteFunction>,
    },
    #[schemars(title = "GetProfile")]
    GetProfile { profile: Option<RemoteProfile> },
    #[schemars(title = "ResolveLatest")]
    ResolveLatest { path: Option<RemotePath> },
}
