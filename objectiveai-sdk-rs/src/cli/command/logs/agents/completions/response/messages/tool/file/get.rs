//! `logs agents completions response messages tool file get` — async handler stub.

use crate::cli::command::IntoCommand;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub id: String,
    pub message_index: u64,
    pub media_index: u64,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv: Vec<String> = vec!["logs", "agents", "completions", "response", "messages", "tool", "file", "get"]
            .into_iter().map(String::from).collect();
        argv.push(self.id.clone());
        argv.push(self.message_index.to_string());
        argv.push(self.media_index.to_string());
        argv
    }
}

pub type Response = crate::agent::completions::message::File;

pub mod request_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}


pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
