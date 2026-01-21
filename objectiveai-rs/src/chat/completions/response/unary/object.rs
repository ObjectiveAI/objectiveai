use crate::chat::completions::response;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum Object {
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
