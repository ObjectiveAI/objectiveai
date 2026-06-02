//! `functions profiles favorites config del` — async handler stub.

use crate::cli::command::IntoCommand;

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub name: String,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["functions".to_string(), "profiles".to_string(), "favorites".to_string(), "config".to_string(), "del".to_string(), self.name.clone()]
    }
}

pub use crate::cli::command::Ok as Response;
