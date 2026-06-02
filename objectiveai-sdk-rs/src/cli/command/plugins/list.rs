//! `plugins list` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
    pub jq: Option<String>,
}

impl CommandRequest for Request {
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
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
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

#[derive(clap::Args)]
pub struct Args {
    /// Skip the first N matching entries.
    #[arg(long)]
    pub offset: Option<usize>,
    /// Return at most N matching entries.
    #[arg(long)]
    pub limit: Option<usize>,
    /// jq filter applied to the JSON output.
    #[arg(long)]
    pub jq: Option<String>,
}

#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Command {
    #[command(flatten)]
    pub args: Args,
    #[command(subcommand)]
    pub schema: Option<Schema>,
}

#[derive(clap::Subcommand)]
pub enum Schema {
    /// Emit the JSON Schema for this leaf's `Request` type and exit.
    RequestSchema(request_schema::Args),
    /// Emit the JSON Schema for this leaf's `Response` type and exit.
    ResponseSchema(response_schema::Args),
}

impl TryFrom<Args> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(args: Args) -> Result<Self, Self::Error> {
        Ok(Self {
            offset: args.offset,
            limit: args.limit,
            jq: args.jq,
        })
    }
}

pub mod request_schema {
    use crate::cli::command::CommandRequest;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    pub struct Request {
        pub jq: Option<String>,
    }

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            let mut argv: Vec<String> = vec!["plugins", "list", "request-schema"].into_iter().map(String::from).collect();
            if let Some(jq) = &self.jq {
                argv.push("--jq".to_string());
                argv.push(jq.clone());
            }
            argv
        }
    }

    pub type Response = schemars::Schema;
}


pub mod response_schema {
    use crate::cli::command::CommandRequest;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    pub struct Request {
        pub jq: Option<String>,
    }

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            let mut argv: Vec<String> = vec!["plugins", "list", "response-schema"].into_iter().map(String::from).collect();
            if let Some(jq) = &self.jq {
                argv.push("--jq".to_string());
                argv.push(jq.clone());
            }
            argv
        }
    }

    pub type Response = schemars::Schema;
}
