//! The request-body `plugins` array.

use crate::agent;
use serde::Serialize;

/// One entry in OpenRouter's request-body `plugins` array. Today the
/// only producer is `context-compression` (see the agent's
/// `context_compression` field), but the shape is OpenRouter-defined
/// — any future plugin id slots in here.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Plugin {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
}

/// The agent's context compression, as the plugin entry it rides in
/// as: the id is the plugin's, and the engine is the enum's own wire
/// string.
impl From<agent::ContextCompression> for Plugin {
    fn from(compression: agent::ContextCompression) -> Self {
        let engine = serde_json::to_value(compression)
            .ok()
            .and_then(|value| value.as_str().map(String::from));
        Plugin {
            id: "context-compression".to_string(),
            engine,
        }
    }
}
