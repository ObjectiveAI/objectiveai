//! `swarms favorites config del` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request {
    pub name: String,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["swarms".to_string(), "favorites".to_string(), "config".to_string(), "del".to_string(), self.name.clone()]
    }
}
