#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid agent: {0}")]
    InvalidAgent(String),

    #[error("invalid continuation")]
    InvalidContinuation,

    #[error("agent not found: {0}")]
    AgentNotFound(String),

    #[error("MCP connection error: {0}")]
    McpConnection(objectiveai_sdk::mcp::Error),

    #[error("MCP connection error: {0}")]
    McpConnectionArc(std::sync::Arc<objectiveai_sdk::mcp::Error>),

    #[error("MCP list_tools error ({url}): {error}")]
    McpListTools {
        url: String,
        error: std::sync::Arc<objectiveai_sdk::mcp::Error>,
    },

    #[error("MCP drain_notifications error ({url}): {error}")]
    McpDrainNotifications {
        url: String,
        error: objectiveai_sdk::mcp::Error,
    },

    /// `read_message_queue` server-request to the CLI failed — either
    /// the WS reverse-attach is down (no live channel, timeout, drop),
    /// or the CLI returned a JSON-RPC error envelope. `clear` failures
    /// are fire-and-forget so don't surface here.
    #[error("read_message_queue via WS reverse-attach failed: {0}")]
    MessageQueueRead(String),

    #[error("MCP has_pending_notifications error ({url}): {error}")]
    McpQueuedNotifications {
        url: String,
        error: objectiveai_sdk::mcp::Error,
    },

    #[error("MCP call_tool error: {0}")]
    McpCallTool(objectiveai_sdk::mcp::Error),

    #[error("MCP proxy bootstrap failed: {0}")]
    McpProxyBootstrap(String),

    #[error(
        "client_objectiveai_mcp is declared but no reverse-attach is available (request must be over WebSocket)"
    )]
    ClientObjectiveaiMcpUnavailable,

    #[error("{0}")]
    Fetch(objectiveai_sdk::error::ResponseError),

    #[error("upstream openrouter error: {0}")]
    UpstreamOpenrouter(Box<dyn super::UpstreamError>),

    #[error("upstream claude agent sdk error: {0}")]
    UpstreamClaudeAgentSdk(Box<dyn super::UpstreamError>),

    #[error("upstream codex sdk error: {0}")]
    UpstreamCodexSdk(Box<dyn super::UpstreamError>),

    #[error("upstream mock error: {0}")]
    UpstreamMock(Box<dyn super::UpstreamError>),

    #[error("no agents resolved")]
    NoAgentsResolved,

    #[error("all agents failed: {0:?}")]
    MultipleErrors(Vec<Error>),

    #[error("timeout")]
    Timeout,

    #[error("empty stream")]
    EmptyStream,

    #[error("stream cancelled")]
    StreamCancelled,
}

impl objectiveai_sdk::error::StatusError for Error {
    fn status(&self) -> u16 {
        use objectiveai_sdk::error::StatusError;
        match self {
            Self::InvalidAgent(_) => 400,
            Self::InvalidContinuation => 400,
            Self::AgentNotFound(_) => 404,
            Self::McpConnection(_) | Self::McpConnectionArc(_) => 502,
            Self::McpListTools { .. } => 502,
            Self::McpDrainNotifications { .. } => 502,
            Self::MessageQueueRead(_) => 502,
            Self::McpQueuedNotifications { .. } => 502,
            Self::McpCallTool(_) => 502,
            Self::McpProxyBootstrap(_) => 500,
            Self::ClientObjectiveaiMcpUnavailable => 400,
            Self::Fetch(e) => e.code,
            Self::UpstreamOpenrouter(e)
            | Self::UpstreamClaudeAgentSdk(e)
            | Self::UpstreamCodexSdk(e)
            | Self::UpstreamMock(e) => e.status(),
            Self::NoAgentsResolved => 400,
            Self::MultipleErrors(errors) => {
                errors.first().map(|e| e.status()).unwrap_or(500)
            }
            Self::Timeout => 504,
            Self::EmptyStream => 502,
            Self::StreamCancelled => 499,
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        use objectiveai_sdk::error::StatusError;
        match self {
            Self::Fetch(e) => Some(e.message.clone()),
            Self::UpstreamOpenrouter(e)
            | Self::UpstreamClaudeAgentSdk(e)
            | Self::UpstreamCodexSdk(e)
            | Self::UpstreamMock(e) => e.message(),
            _ => Some(serde_json::Value::String(self.to_string())),
        }
    }
}
