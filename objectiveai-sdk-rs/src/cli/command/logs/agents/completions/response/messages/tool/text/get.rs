//! `logs agents completions response messages tool text get` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub id: String,
    pub message_index: u64,
    pub media_index: Option<u64>,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv: Vec<String> = vec!["logs", "agents", "completions", "response", "messages", "tool", "text", "get"]
            .into_iter().map(String::from).collect();
        argv.push(self.id.clone());
        argv.push(self.message_index.to_string());
        if let Some(media_index) = self.media_index {
            argv.push(media_index.to_string());
        }
        argv
    }
}

pub type Response = String;

pub mod request_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}


pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
