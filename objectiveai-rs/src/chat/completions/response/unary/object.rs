//! Object type for unary chat completion responses.

use crate::chat::completions::response;
use serde::{Deserialize, Serialize};

/// The object type for chat completion responses.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum Object {
    /// A chat completion object.
    #[serde(rename = "chat.completion")]
    #[default]
    ChatCompletion,
}

impl From<response::streaming::Object> for Object {
    fn from(value: response::streaming::Object) -> Self {
        match value {
            response::streaming::Object::ChatCompletionChunk => {
                Object::ChatCompletion
            }
        }
    }
}
