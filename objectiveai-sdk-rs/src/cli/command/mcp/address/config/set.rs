//! `mcp address config set` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub value: String,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["mcp".to_string(), "address".to_string(), "config".to_string(), "set".to_string(), self.value.clone()]
    }
}

pub type Response = crate::cli::command::Ok;

pub mod request_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}


pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
