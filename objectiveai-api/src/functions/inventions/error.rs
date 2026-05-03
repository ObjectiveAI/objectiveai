use objectiveai::error::StatusError;

/// Errors that can occur during Function invention.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error from agent completions.
    #[error("agent completions error: {0}")]
    AgentCompletions(#[from] crate::agent::completions::Error),
    /// The invention state is invalid.
    #[error("invalid state: {0}")]
    InvalidState(String),
    /// The name already exists and overwrite is not enabled.
    #[error("name already exists: {0}")]
    NameAlreadyExists(String),
    /// Overwrite was requested but is forbidden by server configuration.
    #[error("overwrite forbidden")]
    OverwriteForbidden,
    /// GitHub token validation failed.
    #[error("github token error: {0}")]
    GithubToken(#[from] crate::github::Error),
    /// GitHub token lacks required permissions.
    #[error("github token missing permissions: {0}")]
    GithubTokenMissingPermissions(String),
    /// The name is invalid (too long or would exceed limits with child paths).
    #[error("invalid name: {0}")]
    InvalidName(String),
    /// The remote state was not found.
    #[error("state not found")]
    StateNotFound,
    /// Filesystem error.
    #[error("filesystem error: {0}")]
    Filesystem(#[from] crate::filesystem::Error),
    /// Error fetching a child function referenced by a branch task.
    #[error("function fetch error: {0}")]
    FunctionFetch(objectiveai::error::ResponseError),
    /// The prompt does not support the required type.
    #[error("prompt does not support type: {0}")]
    PromptUnsupportedType(String),
    /// Error fetching the prompt.
    #[error("prompt fetch error: {0}")]
    PromptFetch(objectiveai::error::ResponseError),
    /// Listing tools through the agent's MCP connection failed while
    /// waiting for the InventionServer's tool swap to propagate.
    #[error("mcp list_tools error during tool subscription: {0}")]
    McpListTools(String),
    /// The agent's MCP connection did not show every expected tool
    /// after the InventionServer's `set_tools` call within
    /// `subscribe_tools_timeout`.
    #[error(
        "timed out waiting for invention tools to become visible: \
         expected {expected:?}, observed {observed:?}"
    )]
    ToolSubscriptionTimeout {
        expected: Vec<String>,
        observed: Vec<String>,
    },
}

impl StatusError for Error {
    fn status(&self) -> u16 {
        match self {
            Error::AgentCompletions(e) => e.status(),
            Error::InvalidState(_) => 400,
            Error::NameAlreadyExists(_) => 409,
            Error::OverwriteForbidden => 403,
            Error::GithubToken(e) => e.status(),
            Error::GithubTokenMissingPermissions(_) => 403,
            Error::StateNotFound => 404,
            Error::InvalidName(_) => 400,
            Error::Filesystem(e) => e.status(),
            Error::FunctionFetch(e) => e.code,
            Error::PromptUnsupportedType(_) => 400,
            Error::PromptFetch(e) => e.code,
            Error::McpListTools(_) => 502,
            Error::ToolSubscriptionTimeout { .. } => 504,
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        let error_value = match self {
            Error::AgentCompletions(e) => serde_json::json!({
                "kind": "agent_completions",
                "error": e.message(),
            }),
            Error::InvalidState(msg) => serde_json::json!({
                "kind": "invalid_state",
                "error": msg,
            }),
            Error::NameAlreadyExists(name) => serde_json::json!({
                "kind": "name_already_exists",
                "error": format!("Repository '{}' already exists. Set overwrite to true to allow this.", name),
            }),
            Error::OverwriteForbidden => serde_json::json!({
                "kind": "overwrite_forbidden",
                "error": "Overwrite is forbidden by server configuration.",
            }),
            Error::GithubToken(e) => serde_json::json!({
                "kind": "github_token",
                "error": e.message(),
            }),
            Error::GithubTokenMissingPermissions(msg) => serde_json::json!({
                "kind": "github_token_missing_permissions",
                "error": msg,
            }),
            Error::StateNotFound => serde_json::json!({
                "kind": "state_not_found",
                "error": "remote state not found",
            }),
            Error::InvalidName(msg) => serde_json::json!({
                "kind": "invalid_name",
                "error": msg,
            }),
            Error::Filesystem(e) => serde_json::json!({
                "kind": "filesystem",
                "error": e.message(),
            }),
            Error::FunctionFetch(e) => serde_json::json!({
                "kind": "function_fetch",
                "error": e.message,
            }),
            Error::PromptUnsupportedType(msg) => serde_json::json!({
                "kind": "prompt_unsupported_type",
                "error": msg,
            }),
            Error::PromptFetch(e) => serde_json::json!({
                "kind": "prompt_fetch",
                "error": e.message,
            }),
            Error::McpListTools(msg) => serde_json::json!({
                "kind": "mcp_list_tools",
                "error": msg,
            }),
            Error::ToolSubscriptionTimeout { expected, observed } => serde_json::json!({
                "kind": "tool_subscription_timeout",
                "expected": expected,
                "observed": observed,
            }),
        };
        Some(serde_json::json!({
            "kind": "invention",
            "error": error_value,
        }))
    }
}
