//! `functions get` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.get.Request")]
pub struct Request {
    pub path_type: Path,
    pub path: crate::RemotePathCommitOptional,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.get.Path")]
pub enum Path {
    #[serde(rename = "functions/get")]
    FunctionsGet,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// Response for `functions get` — a resolved Function with its remote path.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.get.Response")]
pub struct Response {
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<crate::RemotePath>")]
    pub path: crate::RemotePath,
    #[serde(flatten)]
    #[schemars(
        schema_with = "crate::flatten_schema::<crate::functions::FullRemoteFunction>"
    )]
    pub inner: crate::functions::FullRemoteFunction,
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("path_required").required(true).args(["path"])))]
pub struct Args {
    /// Path to the target entry.
    #[arg(long)]
    pub path: Option<String>,
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
        let path = args
            .path
            .ok_or_else(|| {
                crate::cli::command::FromArgsError::path_parse(
                    "path",
                    "--path is required".to_string(),
                )
            })?
            .parse::<crate::RemotePathCommitOptional>()
            .map_err(|msg| crate::cli::command::FromArgsError::path_parse("path", msg))?;
        Ok(Self { path_type: Path::FunctionsGet, path, base: args.base.into() })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<Response, E::Error> {
    request.base.clear_transform();
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    transform: crate::cli::command::Transform,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<serde_json::Value, E::Error> {
    request.base.set_transform(transform);
    executor.execute_one(request, agent_arguments).await
}

pub mod request_schema;

pub mod response_schema;

/// One `/listen` broadcast run of `functions get`: the actual
/// [`Request`], the producer's
/// [`AgentArguments`](crate::cli::command::AgentArguments), and the
/// unary response future. See [`crate::cli::websocket_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::websocket_listener::UnaryResponse<Response>,
}
