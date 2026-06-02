//! `agents favorites config get` — async handler stub.

use crate::cli::command::CommandRequest;

pub struct Request;

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        vec![
            "agents".to_string(),
            "favorites".to_string(),
            "config".to_string(),
            "get".to_string(),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ResponseItem {
    pub name: String,
    #[serde(flatten)]
    pub path: crate::RemotePathCommitOptional,
    pub note: String,
}

pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
