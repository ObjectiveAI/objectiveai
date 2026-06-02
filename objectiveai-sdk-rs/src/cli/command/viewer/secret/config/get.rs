//! `viewer secret config get` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request {
    pub filter: Option<String>,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["viewer".to_string(), "secret".to_string(), "config".to_string(), "get".to_string()];
        if let Some(filter) = &self.filter {
            argv.push(filter.clone());
        }
        argv
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}
