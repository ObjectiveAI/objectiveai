use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum Object {
    #[serde(rename = "chat.completion.chunk")]
    #[default]
    ChatCompletionChunk,
}
