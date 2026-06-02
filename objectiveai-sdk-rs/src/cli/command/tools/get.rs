//! `tools get` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request {
    pub name: String,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["tools".to_string(), "get".to_string(), self.name.clone()]
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<serde_json::Value>,
}
