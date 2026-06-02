//! `logs functions executions request get` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub id: String,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv: Vec<String> = vec!["logs", "functions", "executions", "request", "get"]
            .into_iter().map(String::from).collect();
        argv.push(self.id.clone());
        argv
    }
}

pub type Response = crate::functions::executions::request::FunctionExecutionCreateParamsLog;

pub mod request_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}


pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
