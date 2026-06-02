//! `logs agents completions request notifications video get` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub response_id: String,
    pub index: u64,
    pub media_index: u64,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv: Vec<String> = vec!["logs", "agents", "completions", "request", "notifications", "video", "get"]
            .into_iter().map(String::from).collect();
        argv.push(self.response_id.clone());
        argv.push(self.index.to_string());
        argv.push(self.media_index.to_string());
        argv
    }
}

pub type Response = crate::agent::completions::message::VideoUrl;

pub mod request_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}


pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
