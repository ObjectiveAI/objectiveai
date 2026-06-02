//! `update` — async handler stub.

use crate::cli::command::IntoCommand;

pub struct Request;

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        vec!["update".to_string()]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Response {
    Checking {
        asset_name: String,
        current_version: String,
    },
    Found {
        current_version: String,
        remote_version: String,
        asset_name: String,
        url: String,
    },
    Installed {
        current_version: String,
        remote_version: String,
    },
    Skipped {
        reason: ResponseSkipReason,
    },
    UpToDate {
        current_version: String,
        remote_version: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResponseSkipReason {
    DevTree,
    UnsupportedPlatform,
    IncompleteRelease,
}

pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
