//! `logs vector completions response list` — async handler stub.

use crate::cli::command::IntoCommand;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv: Vec<String> = vec!["logs", "vector", "completions", "response", "list"]
            .into_iter().map(String::from).collect();
        if let Some(offset) = self.offset {
            argv.push("--offset".to_string());
            argv.push(offset.to_string());
        }
        if let Some(limit) = self.limit {
            argv.push("--limit".to_string());
            argv.push(limit.to_string());
        }
        argv
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ResponseItem {
    pub id: String,
    pub created: u64,
}

pub mod request_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}


pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
