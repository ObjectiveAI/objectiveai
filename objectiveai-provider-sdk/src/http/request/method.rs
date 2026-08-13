//! The method of a tunneled request.

use serde::{Deserialize, Serialize};

/// What a request is DOING.
///
/// Only what the protocols carried here actually use. This is not a
/// model of HTTP's method set — a method nothing sends is a method
/// nothing has to implement, and adding one later is a variant on the
/// end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Method {
    /// Send a body. In MCP, one JSON-RPC message; the answer is either
    /// one JSON document or an event stream, depending on whether the
    /// far server wants to report progress before its result.
    Post,
    /// Fetch. No body. In MCP this opens the server-initiated event
    /// stream, held open for the life of the SESSION; against a
    /// registry it fetches a manifest or a blob, and may be ranged.
    Get,
    /// Fetch the head of a response and not its body. No body of its
    /// own. A registry runtime uses this constantly, to ask whether a
    /// blob exists before deciding to want it.
    Head,
    /// Terminate. No body. In MCP this ends the session named by
    /// `Mcp-Session-Id`.
    Delete,
}
