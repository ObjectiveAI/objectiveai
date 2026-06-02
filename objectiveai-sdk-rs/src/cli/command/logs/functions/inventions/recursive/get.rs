//! `logs functions inventions recursive get` — async handler stub.

use crate::cli::command::IntoCommand;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub id: String,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv: Vec<String> = vec!["logs", "functions", "inventions", "recursive", "get"]
            .into_iter().map(String::from).collect();
        argv.push(self.id.clone());
        argv
    }
}

pub type Response = crate::functions::inventions::recursive::response::streaming::FunctionInventionRecursiveChunkLog;
