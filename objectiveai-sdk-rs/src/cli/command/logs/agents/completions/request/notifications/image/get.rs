//! `logs agents completions request notifications image get` — async handler stub.

use crate::cli::command::IntoCommand;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub response_id: String,
    pub index: u64,
    pub media_index: u64,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv: Vec<String> = vec!["logs", "agents", "completions", "request", "notifications", "image", "get"]
            .into_iter().map(String::from).collect();
        argv.push(self.response_id.clone());
        argv.push(self.index.to_string());
        argv.push(self.media_index.to_string());
        argv
    }
}

pub type Response = crate::agent::completions::message::ImageUrl;
