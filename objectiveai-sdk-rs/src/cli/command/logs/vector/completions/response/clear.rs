//! `logs vector completions response clear` — async handler stub.

use crate::cli::command::CommandRequest;

pub struct Request;

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv: Vec<String> = vec!["logs", "vector", "completions", "response", "clear"]
            .into_iter().map(String::from).collect();
        argv
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Response {
    pub count: u64,
}

pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
