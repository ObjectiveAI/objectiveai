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

    /// Establishing the MCP connection to the in-container proxy
    /// failed.
    #[error("connecting to the MCP proxy failed: {0}")]
    Connect(#[from] rmcp::service::ClientInitializeError),

    /// The proxy could not list the tools.
    #[error("listing tools failed: {0}")]
    ListTools(rmcp::ServiceError),

    /// The history would not serialize into a continuation token —
    /// which plain data never fails to do, so this names a bug rather
    /// than a circumstance.
    #[error("the continuation would not serialize: {0}")]
    Tokenize(serde_json::Error),

    /// A tool call could not be carried at all — the MCP link itself
    /// failed. The proxy converts every tool-level failure into a
    /// tool response, so this is never a tool merely refusing.
    #[error("a tool call failed: {0}")]
    CallTool(rmcp::ServiceError),
}
