//! `functions favorites config get` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request;

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["functions".to_string(), "favorites".to_string(), "config".to_string(), "get".to_string()]
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResponseFavorite {
    pub name: String,
    #[serde(flatten)]
    pub path: crate::RemotePathCommitOptional,
    pub note: String,
}

pub type Response = Vec<ResponseFavorite>;
