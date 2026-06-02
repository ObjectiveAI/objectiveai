//! `logs agents completions response messages tool video get` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub id: String,
    pub message_index: u64,
    pub media_index: u64,
    pub jq: Option<String>,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv: Vec<String> = vec!["logs", "agents", "completions", "response", "messages", "tool", "video", "get"]
            .into_iter().map(String::from).collect();
        argv.push(self.id.clone());
        argv.push(self.message_index.to_string());
        argv.push(self.media_index.to_string());
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

pub type Response = crate::agent::completions::message::VideoUrl;

pub mod request_schema {
    use crate::cli::command::CommandRequest;

    pub struct Request;

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            vec!["logs", "agents", "completions", "response", "messages", "tool", "video", "get", "--request-schema"].into_iter().map(String::from).collect()
        }
    }

    pub type Response = schemars::Schema;
}


pub mod response_schema {
    use crate::cli::command::CommandRequest;

    pub struct Request;

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            vec!["logs", "agents", "completions", "response", "messages", "tool", "video", "get", "--response-schema"].into_iter().map(String::from).collect()
        }
    }

    pub type Response = schemars::Schema;
}
