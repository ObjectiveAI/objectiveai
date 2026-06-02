//! `logs agents completions response continuations subscribe` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub id: String,
    pub timeout_ms: u64,
    pub require_modification: bool,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv: Vec<String> = vec!["logs", "agents", "completions", "response", "continuations", "subscribe"]
            .into_iter().map(String::from).collect();
        argv.push(self.id.clone());
        argv.push(self.timeout_ms.to_string());
        if self.require_modification {
            argv.push("--require-modification".to_string());
        }
        argv
    }
}

pub type Response = String;

pub mod request_schema {
    use crate::cli::command::CommandRequest;

    pub struct Request;

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            vec!["logs", "agents", "completions", "response", "continuations", "subscribe", "--request-schema"].into_iter().map(String::from).collect()
        }
    }

    pub type Response = schemars::Schema;
}


pub mod response_schema {
    use crate::cli::command::CommandRequest;

    pub struct Request;

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            vec!["logs", "agents", "completions", "response", "continuations", "subscribe", "--response-schema"].into_iter().map(String::from).collect()
        }
    }

    pub type Response = schemars::Schema;
}
