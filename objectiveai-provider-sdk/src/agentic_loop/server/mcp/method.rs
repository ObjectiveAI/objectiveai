//! The method of an MCP request.

use serde::{Deserialize, Serialize};

/// What an MCP request is DOING.
///
/// Streamable HTTP puts all three on one endpoint, so this — not the
/// path — is what distinguishes them. Each has its own shape of
/// answer, which is why the response side cannot assume anything until
/// its head arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum McpMethod {
    /// Send a JSON-RPC message. The body is that message. The answer
    /// is either one JSON document or an event stream, depending on
    /// whether the far server wants to report progress before its
    /// result.
    Post,
    /// Open the server-initiated event stream. No body. The answer is
    /// an event stream held open for the life of the SESSION, which
    /// the far server pushes notifications down whenever it likes.
    Get,
    /// Terminate the session named by `Mcp-Session-Id`. No body.
    Delete,
}
