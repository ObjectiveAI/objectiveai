//! `mcp address config set` — async handler stub.

use crate::cli::command::IntoCommand;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub value: String,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["mcp".to_string(), "address".to_string(), "config".to_string(), "set".to_string(), self.value.clone()]
    }
}

pub use crate::cli::command::Ok as Response;
