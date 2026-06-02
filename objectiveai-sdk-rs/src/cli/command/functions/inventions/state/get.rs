//! `functions inventions state get` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub filter: Option<String>,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["functions".to_string(), "inventions".to_string(), "state".to_string(), "get".to_string()];
        if let Some(filter) = &self.filter {
            argv.push(filter.clone());
        }
        argv
    }
}

pub type Response = crate::functions::inventions::state::response::GetFunctionInventionStateResponse;

pub mod request_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}


pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
