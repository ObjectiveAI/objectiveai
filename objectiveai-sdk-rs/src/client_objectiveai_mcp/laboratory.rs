//! Laboratory-manager channel envelopes.
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
//! The daemon reaches connected laboratories in-process: the conduit and
//! the `laboratories` commands call the resident laboratory registry
//! directly (`LaboratoryRegistry::forward` / `::list`), which forwards
//! over the `/laboratory` WS and correlates the reply. Local and remote
//! laboratories are therefore one code path: whatever dialed
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
    /// Unix seconds when the laboratory container was created, from
    /// podman's own container record. Optional + defaulted so frames
    /// from managers predating this field still parse.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub created_at: Option<i64>,
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

/// The manager's connection-lock key for `(id, address)`:
/// `<id>.<base62(xxh3_128(address))>`. One manager per laboratory per
/// daemon address, enforced by the state's lockfile dir — the address
/// folds to a fixed 22-char base62 token (the agent-id encoding, see
/// `agent::claude_agent_sdk`) so any `ws://` URL is filesystem-safe,
/// and base62 contains no `.`, so the key is unambiguous. Shared by
/// the manager (acquire) and the CLI (spawn-until-published), so the
/// two sides can never disagree on the key.
pub fn connect_lock_key(id: &str, address: &str) -> String {
    let mut hasher = twox_hash::XxHash3_128::with_seed(0);
    hasher.write(address.as_bytes());
    let token = format!("{:0>22}", base62::encode(hasher.finish_128()));
    format!("{id}.{token}")
}

/// Invert [`connect_lock_key`]'s SHAPE: split a lock key into
/// `(id, token)` iff it ends with `.` + exactly 22 base62 characters
/// (the fixed-length address token — base62 contains no `.`).
/// Returns `None` for anything else, notably the bare-id GUARD keys
/// that share the locks directory. One pathological ambiguity exists
/// by construction — a lab id literally ending in `.` + 22 base62
/// chars parses as another lab's connection key — and its failure
/// direction is conservative (the cleaner under-stops); ids in
/// practice never look like that.
pub fn parse_connect_lock_key(key: &str) -> Option<(&str, &str)> {
    let (id, token) = key.rsplit_once('.')?;
    if id.is_empty()
        || token.len() != 22
        || !token.bytes().all(|b| b.is_ascii_alphanumeric())
    {
        return None;
    }
    Some((id, token))
}
