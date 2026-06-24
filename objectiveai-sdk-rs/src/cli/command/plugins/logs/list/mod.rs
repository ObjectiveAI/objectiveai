//! `plugins logs list` — read captured plugin stderr lines for one
//! plugin coordinate (owner/name/version), ascending by the BIGSERIAL
//! `"index"` cursor (paginate with `--after-id` / `--limit`).

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.logs.list.Request")]
pub struct Request {
    pub path_type: Path,
    pub owner: String,
    pub name: String,
    pub version: String,
    /// Skip rows with `"index" <= after_id`. Use the highest `index`
    /// from a previous page to paginate forward.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub after_id: Option<i64>,
    /// Cap on rows returned. `None` = unlimited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub limit: Option<i64>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.logs.list.Path")]
pub enum Path {
    #[serde(rename = "plugins/logs/list")]
    PluginsLogsList,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// One captured stderr line from a `plugins run` invocation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.logs.list.ResponseItem")]
pub struct ResponseItem {
    /// `objectiveai.plugin_messages."index"` — the BIGSERIAL cursor;
    /// pass as `--after-id` to page forward.
    pub index: i64,
    pub owner: String,
    pub name: String,
    pub version: String,
    /// AIH of the agent run that invoked `plugins run`.
    pub agent_instance_hierarchy: String,
    /// Response id of the invoking agent run, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub response_id: Option<String>,
    /// Unix-seconds capture time.
    pub created_at: i64,
    pub line: String,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("owner_required").required(true).args(["owner"])))]
#[command(group(clap::ArgGroup::new("name_required").required(true).args(["name"])))]
#[command(group(clap::ArgGroup::new("version_required").required(true).args(["version"])))]
pub struct Args {
    /// Plugin owner (GitHub `<owner>` segment). Required.
    #[arg(long)]
    pub owner: Option<String>,
    /// Plugin name (repository segment). Required.
    #[arg(long)]
    pub name: Option<String>,
    /// Plugin version. Required.
    #[arg(long)]
    pub version: Option<String>,
    /// Skip rows with `"index" <= after_id`.
    #[arg(long)]
    pub after_id: Option<i64>,
    /// Cap on rows returned.
    #[arg(long)]
    pub limit: Option<i64>,
    #[command(flatten)]
    pub base: crate::cli::command::RequestBaseArgs,
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
            path_type: Path::PluginsLogsList,
            owner: args.owner.ok_or_else(|| {
                crate::cli::command::FromArgsError::path_parse(
                    "owner",
                    "--owner is required".to_string(),
                )
            })?,
            name: args.name.ok_or_else(|| {
                crate::cli::command::FromArgsError::path_parse(
                    "name",
                    "--name is required".to_string(),
                )
            })?,
            version: args.version.ok_or_else(|| {
                crate::cli::command::FromArgsError::path_parse(
                    "version",
                    "--version is required".to_string(),
                )
            })?,
            after_id: args.after_id,
            limit: args.limit,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<E::Stream<ResponseItem>, E::Error> {
    request.base.clear_transform();
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    transform: crate::cli::command::Transform,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<E::Stream<serde_json::Value>, E::Error> {
    request.base.set_transform(transform);
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;

pub mod response_schema;
