//! `plugins install github` — async handler stub.

use crate::cli::command::IntoCommand;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub owner: String,
    pub repository: String,
    pub commit_sha: Option<String>,
    pub allow_untrusted: bool,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "plugins".to_string(),
            "install".to_string(),
            "github".to_string(),
            "--owner".to_string(),
            self.owner.clone(),
            "--repository".to_string(),
            self.repository.clone(),
        ];
        if let Some(sha) = &self.commit_sha {
            argv.push("--commit-sha".to_string());
            argv.push(sha.clone());
        }
        if self.allow_untrusted {
            argv.push("--allow-untrusted".to_string());
        }
        argv
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Response {
    pub installed: bool,
}
