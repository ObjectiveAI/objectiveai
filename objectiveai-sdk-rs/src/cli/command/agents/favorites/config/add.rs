//! `agents favorites config add` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request {
    pub name: String,
    pub path: String,
    pub note: String,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec![
            "agents".to_string(),
            "favorites".to_string(),
            "config".to_string(),
            "add".to_string(),
            "--name".to_string(),
            self.name.clone(),
            "--path".to_string(),
            self.path.clone(),
            "--note".to_string(),
            self.note.clone(),
        ]
    }
}

pub use crate::cli::command::Ok as Response;
