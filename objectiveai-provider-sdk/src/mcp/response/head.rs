//! The head of an MCP response.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::decode::Decode;

/// The status and headers of an MCP response.
///
/// The first thing back, and never repeated — see
/// [`Frame`](super::Frame) for why it arrives separately from the body
/// it introduces.
///
/// A type rather than a pair of fields on that variant, so there is
/// something to point serde at. This is the one part of an MCP
/// exchange that IS a shape rather than a stream: the bodies around it
/// are bytes nobody parses, and this is a JSON object of exactly these
/// two fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Head {
    /// The HTTP status.
    ///
    /// Load-bearing, and the reason a bare JSON-RPC message would not
    /// do: `202` marks a notification that has no body coming, and
    /// `404` tells an agent its session is gone and must be
    /// re-initialized. Neither fact has anywhere to live inside
    /// JSON-RPC.
    pub status: u16,
    /// The response headers, verbatim.
    ///
    /// Also load-bearing. `Mcp-Session-Id` is how an agent LEARNS its
    /// session id in the first place — the initialize response mints
    /// it — and `Content-Type` is what tells the agent whether it is
    /// reading one JSON document or an event stream.
    ///
    /// A map, so a header name appears at most once. This is the
    /// direction where that could bite, since `Set-Cookie` is the
    /// classic repeated header; MCP does not use cookies, and the
    /// ergonomics everywhere else are worth more than the case.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub headers: IndexMap<String, String>,
}

impl Decode<'_> for Head {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        serde_json::from_slice(bytes)
    }
}
