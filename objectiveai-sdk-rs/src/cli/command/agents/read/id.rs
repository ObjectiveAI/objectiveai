//! `agents read id` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request {
    pub id: i64,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec![
            "agents".to_string(),
            "read".to_string(),
            "id".to_string(),
            self.id.to_string(),
        ]
    }
}
