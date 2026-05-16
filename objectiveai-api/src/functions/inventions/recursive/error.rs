use objectiveai_sdk::error::StatusError;

/// Errors that can occur during recursive Function invention.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error from the single-level invention client.
    #[error("invention error: {0}")]
    Invention(#[from] crate::functions::inventions::Error),
    /// The first chunk of the invention stream contained an error.
    #[error("invention failed: {0}")]
    InventionFirstChunk(objectiveai_sdk::error::ResponseError),
}

impl StatusError for Error {
    fn status(&self) -> u16 {
        match self {
            Error::Invention(e) => e.status(),
            Error::InventionFirstChunk(e) => e.code,
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        let error_value = match self {
            Error::Invention(e) => serde_json::json!({
                "kind": "invention",
                "error": e.message(),
            }),
            Error::InventionFirstChunk(e) => serde_json::json!({
                "kind": "invention_first_chunk",
                "error": e.message,
            }),
        };
        Some(serde_json::json!({
            "kind": "recursive_invention",
            "error": error_value,
        }))
    }
}
