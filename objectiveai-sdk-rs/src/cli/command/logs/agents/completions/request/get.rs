//! `logs agents completions request get` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub id: String,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv: Vec<String> = vec!["logs", "agents", "completions", "request", "get"]
            .into_iter().map(String::from).collect();
        argv.push(self.id.clone());
        argv
    }
}

pub type Response = crate::agent::completions::request::AgentCompletionCreateParamsLog;

pub mod request_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}


pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
