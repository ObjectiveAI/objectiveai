#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Script agents run on the CLIENT: without a WS-attached client
    /// there is nowhere to run the code (the SSE/unary path has no
    /// reverse channel).
    #[error("script upstream requires a WebSocket-attached client")]
    ReverseChannelUnavailable,

    #[error("reverse channel closed before the script request was sent")]
    ChannelClosed,

    #[error("reverse channel dropped before the script replied")]
    ChannelDropped,

    #[error("script error {code}: {message}")]
    Script { code: i64, message: String },

    #[error("client answered the script request with the wrong variant")]
    WrongVariant,

    #[error("script output no messages — it must output a non-empty messages array (assistant/tool roles only)")]
    EmptyOutput,
}

impl objectiveai_sdk::error::StatusError for Error {
    fn status(&self) -> u16 {
        match self {
            Self::ReverseChannelUnavailable => 502,
            Self::ChannelClosed => 502,
            Self::ChannelDropped => 502,
            Self::Script { .. } => 502,
            Self::WrongVariant => 502,
            Self::EmptyOutput => 502,
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        Some(serde_json::Value::String(self.to_string()))
    }
}
