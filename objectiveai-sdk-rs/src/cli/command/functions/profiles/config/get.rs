//! `functions profiles config get` — async handler stub.

use crate::cli::command::IntoCommand;

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub filter: Option<String>,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["functions".to_string(), "profiles".to_string(), "config".to_string(), "get".to_string()];
        if let Some(filter) = &self.filter {
            argv.push(filter.clone());
        }
        argv
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favorites: Option<Vec<super::favorites::config::get::ResponseItem>>,
}
