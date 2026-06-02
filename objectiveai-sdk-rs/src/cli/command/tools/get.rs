//! `tools get` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub name: String,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["tools".to_string(), "get".to_string(), self.name.clone()]
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ResponseManifest {
    pub name: String,
    pub description: String,
    pub version: String,
    pub owner: String,
    pub exec: String,
    pub source: String,
}

pub type Response = Option<ResponseManifest>;

pub mod request_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}


pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
