//! What can go wrong across a whole loop.

use crate::fetch;

/// A loop that failed — in its own machinery, or in the fetch under
/// it.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The OpenRouter call failed; every fetch failure is a loop
    /// failure.
    #[error(transparent)]
    Fetch(#[from] fetch::Error),

    /// The proxy's MCP session could not be made.
    #[error("connecting to the proxy's MCP server failed: {0}")]
    Connect(diverge_container_proxy_sdk::Error),

    /// The proxy could not list the tools.
    #[error("listing tools failed: {0}")]
    ListTools(rmcp::ServiceError),

    /// A tool call could not be carried at all — the MCP link itself
    /// failed. The proxy converts every tool-level failure into a
    /// tool response, so this is never a tool merely refusing.
    #[error("a tool call failed: {0}")]
    CallTool(rmcp::ServiceError),
}

impl Error {
    /// The failure as JSON, in the same shape fetch reports — a
    /// notification's message, or the error frame's, depending on
    /// whether the loop had already spoken.
    pub fn message(&self) -> serde_json::Value {
        match self {
            Error::Fetch(error) => error.message(),
            Error::Connect(error) => serde_json::json!({
                "kind": "loop",
                "error": {
                    "kind": "connect",
                    "error": error.to_string(),
                },
            }),
            Error::ListTools(error) => serde_json::json!({
                "kind": "loop",
                "error": {
                    "kind": "list_tools",
                    "error": error.to_string(),
                },
            }),
            Error::CallTool(error) => serde_json::json!({
                "kind": "loop",
                "error": {
                    "kind": "call_tool",
                    "error": error.to_string(),
                },
            }),
        }
    }
}
