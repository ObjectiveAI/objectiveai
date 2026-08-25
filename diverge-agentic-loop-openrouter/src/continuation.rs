//! What an OpenRouter continuation token holds.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use diverge_provider_sdk::endpoints::agentic_loop::run::server::response::AgenticLoopChunk;

/// A continuation token, opened.
///
/// The token is opaque to everyone but this container: what it holds
/// is the loop's own history — the chunks the run produced, in order —
/// as a JSON array, base64-ified. Resuming is deserializing it and
/// rebuilding the conversation from what was already said.
#[derive(Debug, Clone, PartialEq)]
pub struct Continuation(pub Vec<AgenticLoopChunk>);

impl Continuation {
    /// Open a raw continuation string: un-base64 it, deserialize it.
    pub fn new(token: &str) -> Result<Self, ContinuationError> {
        let json = STANDARD
            .decode(token)
            .map_err(ContinuationError::Base64)?;
        serde_json::from_slice(&json)
            .map(Continuation)
            .map_err(ContinuationError::Json)
    }
}

/// A continuation token that could not be opened.
///
/// Two layers, two failures: the coat did not decode, or what was
/// inside was not the history this container writes.
#[derive(Debug)]
pub enum ContinuationError {
    /// The token is not base64.
    Base64(base64::DecodeError),
    /// The decoded bytes are not the JSON history.
    Json(serde_json::Error),
}

impl std::fmt::Display for ContinuationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContinuationError::Base64(error) => {
                write!(f, "a continuation token is not base64: {error}")
            }
            ContinuationError::Json(error) => {
                write!(f, "a continuation token did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for ContinuationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ContinuationError::Base64(error) => Some(error),
            ContinuationError::Json(error) => Some(error),
        }
    }
}
