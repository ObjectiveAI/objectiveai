//! `viewer generate-secret-signature-pair` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request;

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["viewer".to_string(), "generate-secret-signature-pair".to_string()]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Response {
    pub secret: String,
    pub signature: String,
}

pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
