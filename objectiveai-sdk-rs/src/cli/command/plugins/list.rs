//! `plugins list` — async handler stub.

use crate::cli::command::IntoCommand;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["plugins".to_string(), "list".to_string()];
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ResponseItem {
    pub name: String,
    pub description: String,
    pub version: String,
    pub owner: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "ResponseBinaries::is_empty")]
    pub binaries: ResponseBinaries,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewer_zip: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewer_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub viewer_routes: Vec<ResponseViewerRoute>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub mobile_ready: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<ResponseMcpServer>,
    pub source: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ResponseBinaries {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linux_x86_64: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linux_aarch64: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows_x86_64: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows_aarch64: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub macos_x86_64: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub macos_aarch64: Option<String>,
}

impl ResponseBinaries {
    fn is_empty(&self) -> bool {
        self.linux_x86_64.is_none()
            && self.linux_aarch64.is_none()
            && self.windows_x86_64.is_none()
            && self.windows_aarch64.is_none()
            && self.macos_x86_64.is_none()
            && self.macos_aarch64.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ResponseViewerRoute {
    pub path: String,
    pub method: ResponseHttpMethod,
    pub r#type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum ResponseHttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ResponseMcpServer {
    pub name: String,
    pub url: String,
    pub authorization: bool,
}

pub mod request_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}


pub mod response_schema {
    pub struct Request;
    pub type Response = schemars::Schema;
}
