//! `agents get` — async handler stub.

use crate::cli::command::IntoCommand;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub path: String,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec![
            "agents".to_string(),
            "get".to_string(),
            "--path".to_string(),
            self.path.clone(),
        ]
    }
}

pub type Response = crate::agent::response::GetAgentResponse;
