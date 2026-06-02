//! `functions profiles favorites config get` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request;

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["functions".to_string(), "profiles".to_string(), "favorites".to_string(), "config".to_string(), "get".to_string()]
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
