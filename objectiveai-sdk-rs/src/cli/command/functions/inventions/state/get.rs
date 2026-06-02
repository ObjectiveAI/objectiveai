//! `functions inventions state get` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request {
    pub filter: Option<String>,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["functions".to_string(), "inventions".to_string(), "state".to_string(), "get".to_string()];
        if let Some(filter) = &self.filter {
            argv.push(filter.clone());
        }
        argv
    }
}
