//! `viewer signature config set` — async handler stub.

use crate::cli::command::IntoCommand;

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub value: String,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["viewer".to_string(), "signature".to_string(), "config".to_string(), "set".to_string(), self.value.clone()]
    }
}

pub use crate::cli::command::Ok as Response;
