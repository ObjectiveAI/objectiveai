//! `functions profiles get` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub path: String,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["functions".to_string(), "profiles".to_string(), "get".to_string(), "--path".to_string(), self.path.clone()]
    }
}

pub type Response = crate::functions::RemoteProfile;

pub mod request_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}


pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
