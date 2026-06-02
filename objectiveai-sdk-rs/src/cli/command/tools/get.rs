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
