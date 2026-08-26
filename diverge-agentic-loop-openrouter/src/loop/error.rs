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
}
