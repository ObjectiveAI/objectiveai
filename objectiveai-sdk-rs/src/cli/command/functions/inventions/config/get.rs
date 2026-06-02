//! `functions inventions config get` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub filter: Option<String>,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["functions".to_string(), "inventions".to_string(), "config".to_string(), "get".to_string()];
        if let Some(filter) = &self.filter {
            argv.push(filter.clone());
        }
        argv
    }
}

#[derive(PartialEq, Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Response {
    pub remote: crate::Remote,
}

pub mod request_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}


pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
