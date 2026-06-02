//! `mcp port config set` — async handler stub.

use crate::cli::command::IntoCommand;

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub value: u16,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["mcp".to_string(), "port".to_string(), "config".to_string(), "set".to_string(), self.value.to_string()]
    }
}

pub use crate::cli::command::Ok as Response;
