//! `agents me` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request;

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["agents".to_string(), "me".to_string()]
    }
}

pub type Response = String;
