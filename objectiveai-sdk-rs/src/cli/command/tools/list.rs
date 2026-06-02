//! `tools list` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["tools".to_string(), "list".to_string()];
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResponseManifest {
    pub name: String,
    pub description: String,
    pub version: String,
    pub owner: String,
    pub exec: String,
    pub source: String,
}

pub type Response = Vec<ResponseManifest>;
