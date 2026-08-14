//! The head of a tunneled response.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// The status and headers of a response.
///
/// The first thing back, and never repeated — see
/// [`Frame`](super::Frame) for why it arrives separately from the body
/// it introduces.
///
/// A type rather than a pair of fields on that variant, so there is
/// something to point serde at. This is the one part of an exchange
/// that IS a shape rather than a stream: the bodies around it are
/// bytes nobody parses, and this is a JSON object of exactly these two
/// fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Head {
    /// The HTTP status.
    ///
    /// Load-bearing, and the reason a bare protocol message would not
    /// do. In MCP, `202` marks a notification with no body coming and
    /// `404` tells an agent its session is gone and must be
    /// re-initialized. Against a registry, `206` is a partial answer
    /// to a ranged request and `404` is a blob that is simply not
    /// there — which is how "I cannot give you that" is said, with no
    /// separate signal needed.
    pub status: u16,
    /// The response headers, verbatim.
    ///
    /// Also load-bearing. `Mcp-Session-Id` is how an agent LEARNS its
    /// session id in the first place — the initialize response mints
    /// it — and `Content-Type` is what tells it whether it is reading
    /// one JSON document or an event stream. A registry answer needs
    /// `Content-Length` and `Content-Range` for the same kind of
    /// reason: a runtime cannot verify what it did not know the size
    /// of.
    ///
    /// A map, so a header name appears at most once. This is the
    /// direction where that could bite, since `Set-Cookie` is the
    /// classic repeated header; neither protocol here uses cookies,
    /// and the ergonomics everywhere else are worth more than the
    /// case.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub headers: IndexMap<String, String>,
}
