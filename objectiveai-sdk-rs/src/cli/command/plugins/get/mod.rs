//! `plugins get` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.get.Request")]
pub struct Request {
    pub path_type: Path,
    pub name: String,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.get.Path")]
pub enum Path {
    #[serde(rename = "plugins/get")]
    PluginsGet,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["plugins".to_string(), "get".to_string(), self.name.clone()];
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

pub type Response = Option<ResponseManifest>;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.get.ResponseManifest")]
pub struct ResponseManifest {
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
#[schemars(rename = "cli.command.plugins.get.ResponseBinaries")]
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
#[schemars(rename = "cli.command.plugins.get.ResponseViewerRoute")]
pub struct ResponseViewerRoute {
    pub path: String,
    pub method: ResponseHttpMethod,
    pub r#type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
#[schemars(rename = "cli.command.plugins.get.ResponseHttpMethod")]
pub enum ResponseHttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.get.ResponseMcpServer")]
pub struct ResponseMcpServer {
    pub name: String,
    pub url: String,
    pub authorization: bool,
}

#[derive(clap::Args)]
pub struct Args {
    /// Plugin/tool name.
    pub name: String,
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
        Ok(Self { path_type: Path::PluginsGet,
            name: args.name,
            jq: args.jq,
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<Response, E::Error> {
    request.jq = None;
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    jq: String,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<serde_json::Value, E::Error> {
    request.jq = Some(jq);
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseManifest {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;


pub mod response_schema;
