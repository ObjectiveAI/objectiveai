//! `swarms favorites config edit` — async handler stub.

use crate::cli::command::CommandRequest;

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

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["swarms".to_string(), "favorites".to_string(), "config".to_string(), "edit".to_string(), self.name.clone()];
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

pub type Response = crate::cli::command::Ok;

pub mod request_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}


pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
