//! Laboratory-manager channel + socket envelopes.
//!
//! A laboratory is managed by a standalone `objectiveai-laboratory`
//! process that dials OUT to the daemon's `/laboratory` WebSocket.
//! The wire there is:
//!
//! 1. **[`Identify`]** — the FIRST text frame, before any
//!    authorization: who this laboratory is (id + container spec).
//! 2. The standard `AuthEnvelope` (`{"signature": …}`) — authorization
//!    strictly FOLLOWS identity.
//! 3. Then a correlated request/response protocol: the daemon sends
//!    [`ChannelRequest`]s (the [`super::server_request::Payload`]
//!    vocabulary, verbatim) and the manager answers with
//!    [`ChannelResponse`]s.
//!
//! The CLI conduit reaches connected laboratories through the daemon's
//! `laboratories.sock` local socket with [`SocketRequest`] /
//! [`SocketResponse`] (one JSON line each way per connection) — the
//! daemon forwards over the WS and correlates the reply. Local and
//! remote laboratories are therefore one code path: whatever dialed
//! `/laboratory` serves the traffic, wherever it runs.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One bind mount in a laboratory's identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.laboratory.IdentifyMount")]
pub struct IdentifyMount {
    pub host: String,
    pub container: String,
}

/// The `/laboratory` connection's FIRST frame: who this laboratory is.
/// Sent BEFORE the `AuthEnvelope` — identity always precedes
/// authorization. Mirrors the `laboratories create` spec so
/// `laboratories list` can echo it verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.laboratory.Identify")]
pub struct Identify {
    /// The RAW, state-agnostic laboratory id — never prefixed or
    /// namespaced (the manager's state scopes its container NAME and
    /// its `<state>/locks/laboratories/<id>` lock, but the identity on
    /// this wire is the bare id). Local-vs-remote classification in
    /// `laboratories list` compares exactly this value against the
    /// local machine's state-scoped container scan.
    pub id: String,
    pub image: String,
    pub mounts: Vec<IdentifyMount>,
    pub env: Vec<[String; 2]>,
    pub cwd: String,
}

/// Daemon → manager over the `/laboratory` WS: one correlated request.
/// `payload` is the reverse-attach vocabulary verbatim — the manager
/// is a mini-conduit for its one laboratory.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.laboratory.ChannelRequest")]
pub struct ChannelRequest {
    /// Correlation id, minted by the daemon; echoed by the response.
    pub id: String,
    /// The originating request's headers (e.g.
    /// `X-OBJECTIVEAI-RESPONSE-ID`, which keys the manager's per-session
    /// MCP connections).
    pub headers: IndexMap<String, String>,
    #[serde(flatten)]
    pub payload: super::server_request::Payload,
}

/// Manager → daemon: the reply to a [`ChannelRequest`], correlated by
/// `id`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.laboratory.ChannelResponse")]
pub struct ChannelResponse {
    pub id: String,
    #[serde(flatten)]
    pub payload: super::server_response::Payload,
}

/// One request line on the daemon's `laboratories.sock` local socket
/// (CLI/conduit → daemon). Exactly one request → one
/// [`SocketResponse`] line per connection.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
#[schemars(rename = "client_objectiveai_mcp.laboratory.SocketRequest")]
pub enum SocketRequest {
    /// Forward `request` to the connected laboratory `laboratory_id`
    /// and relay its reply.
    Forward {
        laboratory_id: String,
        headers: IndexMap<String, String>,
        request: super::server_request::Payload,
    },
    /// Snapshot the identities of every connected laboratory.
    List,
}

/// One response line on `laboratories.sock` (daemon → CLI/conduit).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "client_objectiveai_mcp.laboratory.SocketResponse")]
pub enum SocketResponse {
    Forwarded {
        response: super::server_response::Payload,
    },
    List { laboratories: Vec<Identify> },
    /// Daemon-level failure: unknown laboratory, manager disconnected
    /// mid-request, forward timeout, malformed request line.
    Error { message: String },
}
