//! `viewer port config set` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub value: u16,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["viewer".to_string(), "port".to_string(), "config".to_string(), "set".to_string(), self.value.to_string()]
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
