//! `functions favorites config add` — async handler stub.

use crate::cli::command::IntoCommand;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub name: String,
    pub path: String,
    pub note: String,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec![
            "functions".to_string(), "favorites".to_string(), "config".to_string(), "add".to_string(),
            "--name".to_string(),
            self.name.clone(),
            "--path".to_string(),
            self.path.clone(),
            "--note".to_string(),
            self.note.clone(),
        ]
    }
}

pub type Response = crate::cli::command::Ok;
