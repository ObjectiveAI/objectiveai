#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Expected error")]
    ExpectedError,

    #[error("unsupported response format: {0}")]
    UnsupportedResponseFormat(String),

    #[error("tools not allowed but response format requires a tool call")]
    ToolsNotAllowedWithRequiredToolCall,

    #[error("mock tool call limit exceeded ({0})")]
    MaxToolCallsExceeded(u32),

    #[error("MCP list_tools error ({url}): {error}")]
    McpListTools {
        url: String,
        error: std::sync::Arc<objectiveai_sdk::mcp::Error>,
    },
}

impl objectiveai_sdk::error::StatusError for Error {
    fn status(&self) -> u16 {
        match self {
            Self::ExpectedError => 500,
            Self::UnsupportedResponseFormat(_) => 400,
            Self::ToolsNotAllowedWithRequiredToolCall => 400,
            Self::MaxToolCallsExceeded(_) => 429,
            Self::McpListTools { .. } => 502,
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        Some(serde_json::Value::String(self.to_string()))
    }
}
