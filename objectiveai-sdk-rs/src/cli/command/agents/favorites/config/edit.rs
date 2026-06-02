//! `agents favorites config edit` — async handler stub.

use crate::cli::command::IntoCommand;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub name: String,
    pub note: Option<String>,
    pub commit: Option<RequestCommitChange>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum RequestCommitChange {
    Set(String),
    Remove,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "favorites".to_string(),
            "config".to_string(),
            "edit".to_string(),
            self.name.clone(),
        ];
        if let Some(note) = &self.note {
            argv.push("--note".to_string());
            argv.push(note.clone());
        }
        match &self.commit {
            Some(RequestCommitChange::Set(c)) => {
                argv.push("--commit".to_string());
                argv.push(c.clone());
            }
            Some(RequestCommitChange::Remove) => {
                argv.push("--remove-commit".to_string());
            }
            None => {}
        }
        argv
    }
}

pub use crate::cli::command::Ok as Response;
