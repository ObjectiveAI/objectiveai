//! `update` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.update.Request")]
pub struct Request {
    pub path_type: Path,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.update.Path")]
pub enum Path {
    #[serde(rename = "update")]
    Update,
}
impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["update".to_string()];
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

/// Per-stage update event. Update is a streaming leaf — one
/// `ResponseItem` per (asset, stage) pair as the update progresses
/// through the four shipped binaries (cli, api, viewer, mcp).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.command.update.ResponseItem")]
pub enum ResponseItem {
    #[schemars(title = "Checking")]
    Checking {
        asset_name: String,
        current_version: String,
    },
    #[schemars(title = "Found")]
    Found {
        current_version: String,
        remote_version: String,
        asset_name: String,
        url: String,
    },
    #[schemars(title = "Installed")]
    Installed {
        current_version: String,
        remote_version: String,
    },
    #[schemars(title = "Skipped")]
    Skipped {
        reason: ResponseSkipReason,
    },
    #[schemars(title = "UpToDate")]
    UpToDate {
        current_version: String,
        remote_version: String,
    },
}

/// All-at-once view of an update run. Used only for schema generation
/// (`response-schema`); the leaf's `execute` returns
/// `Stream<ResponseItem>` rather than building a `Response`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.update.Response")]
pub struct Response {
    pub items: Vec<ResponseItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "cli.command.update.ResponseSkipReason")]
pub enum ResponseSkipReason {
    DevTree,
    UnsupportedPlatform,
    IncompleteRelease,
}

#[derive(clap::Args)]
pub struct Args {
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
        Ok(Self { path_type: Path::Update, jq: args.jq })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<E::Stream<ResponseItem>, E::Error> {
    request.jq = None;
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    jq: String,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<E::Stream<serde_json::Value>, E::Error> {
    request.jq = Some(jq);
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;

pub mod response_schema;
