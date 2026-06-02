//! `logs functions executions response subscribe` — async handler stub.

use crate::cli::command::IntoCommand;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub id: String,
    pub timeout_ms: u64,
    pub require_modification: bool,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv: Vec<String> = vec!["logs", "functions", "executions", "response", "subscribe"]
            .into_iter().map(String::from).collect();
        argv.push(self.id.clone());
        argv.push(self.timeout_ms.to_string());
        if self.require_modification {
            argv.push("--require-modification".to_string());
        }
        argv
    }
}

pub type Response = crate::functions::executions::response::streaming::FunctionExecutionChunkLog;

pub mod request_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}


pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
