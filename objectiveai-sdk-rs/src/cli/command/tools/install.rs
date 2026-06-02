//! `tools install` — async handler stub.

use crate::cli::command::CommandRequest;

pub struct Request;

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["tools".to_string(), "install".to_string()]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Response {
    pub instructions: String,
}

pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
