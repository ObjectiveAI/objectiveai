//! `agents read id` — async handler stub.

use crate::cli::command::IntoCommand;

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub id: i64,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec![
            "agents".to_string(),
            "read".to_string(),
            "id".to_string(),
            self.id.to_string(),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Response {
    Json { content: serde_json::Value },
    Text { text: String },
    Image { image_url: crate::agent::completions::message::ImageUrl },
    Audio { input_audio: crate::agent::completions::message::InputAudio },
    Video { video_url: crate::agent::completions::message::VideoUrl },
    File { file: crate::agent::completions::message::File },
}
