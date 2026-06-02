//! `tools run` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request {
    pub name: String,
    pub args: Vec<String>,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "tools".to_string(),
            "run".to_string(),
            self.name.clone(),
        ];
        argv.extend(self.args.iter().cloned());
        argv
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResponseItem {
    pub line: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr: Option<bool>,
}
