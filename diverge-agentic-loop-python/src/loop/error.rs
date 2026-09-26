//! What can go wrong across a whole loop.

use crate::run;

/// A loop that failed — in its own machinery, or in the script under
/// it.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The script's run failed; every run failure is a loop failure.
    #[error(transparent)]
    Run(#[from] run::Error),

    /// The proxy's MCP session could not be made.
    #[error("connecting to the proxy's MCP server failed: {0}")]
    Connect(diverge_sdk::container_proxy::inside::Error),

    /// The proxy could not list the tools.
    #[error("listing tools failed: {0}")]
    ListTools(rmcp::ServiceError),

    /// The proxy could not list the resources.
    #[error("listing resources failed: {0}")]
    ListResources(rmcp::ServiceError),

    /// The script called a tool the list it was fed does not carry.
    /// Not a tool refusing — the proxy would answer that as a tool
    /// response — but a name that names nothing, which is the
    /// script's mistake and ends the run.
    #[error("the script called a tool that does not exist: `{name}` (call `{id}`)")]
    UnknownTool {
        /// The call's id, as the script gave it.
        id: String,
        /// The name nothing answers to.
        name: String,
    },

    /// A tool call could not be carried at all — the MCP link itself
    /// failed. The proxy converts every tool-level failure into a
    /// tool response, so this is never a tool merely refusing.
    #[error("a tool call failed: {0}")]
    CallTool(rmcp::ServiceError),

    /// The task carrying a tool call died before answering.
    #[error("a tool call's task died: {0}")]
    Join(tokio::task::JoinError),
}

impl Error {
    /// The failure as JSON, in the same shape the run reports — a
    /// notification's message, or the error frame's, depending on
    /// whether the loop had already spoken.
    pub fn message(&self) -> serde_json::Value {
        let error = match self {
            Error::Run(error) => return error.message(),
            Error::Connect(error) => serde_json::json!({
                "kind": "connect",
                "error": error.to_string(),
            }),
            Error::ListTools(error) => serde_json::json!({
                "kind": "list_tools",
                "error": error.to_string(),
            }),
            Error::ListResources(error) => serde_json::json!({
                "kind": "list_resources",
                "error": error.to_string(),
            }),
            Error::UnknownTool { id, name } => serde_json::json!({
                "kind": "unknown_tool",
                "id": id,
                "name": name,
            }),
            Error::CallTool(error) => serde_json::json!({
                "kind": "call_tool",
                "error": error.to_string(),
            }),
            Error::Join(error) => serde_json::json!({
                "kind": "join",
                "error": error.to_string(),
            }),
        };
        serde_json::json!({
            "kind": "loop",
            "error": error,
        })
    }
}
