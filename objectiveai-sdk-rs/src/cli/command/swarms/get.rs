//! `swarms get` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request {
    pub path: String,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["swarms".to_string(), "get".to_string(), "--path".to_string(), self.path.clone()]
    }
}
