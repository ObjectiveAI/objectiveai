//! `viewer send` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.viewer.send.Request")]
pub struct Request {
    pub path_type: Path,
    pub path: String,
    pub body: serde_json::Value,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.viewer.send.Path")]
pub enum Path {
    #[serde(rename = "viewer/send")]
    ViewerSend,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// `viewer send` no longer POSTs to the viewer over HTTP — the request
/// is broadcast to the viewer over the daemon WebSocket instead — so it
/// just acknowledges with the shared `Ok` sentinel and returns
/// immediately.
pub type Response = crate::cli::command::Ok;

/// Viewer-stream mirror of [`Request`]: the request (nested under
/// `value`, `path_type` and all) plus the broadcast stream `id`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.viewer.send.ViewerRequest")]
pub struct ViewerRequest {
    pub id: String,
    pub value: Request,
}

/// Viewer-stream mirror of [`Response`]: the response (nested under
/// `value`) plus the broadcast stream `id` and the originating request's
/// `path_type`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.viewer.send.ViewerResponse")]
pub struct ViewerResponse {
    pub id: String,
    pub path_type: Path,
    pub value: Response,
}

fn parse_json_value(s: &str) -> Result<serde_json::Value, serde_json::Error> {
    serde_json::from_str(s)
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("path_required").required(true).args(["path"])))]
#[command(group(clap::ArgGroup::new("body_required").required(true).args(["body"])))]
pub struct Args {
    /// HTTP path on the viewer to POST to.
    #[arg(long)]
    pub path: Option<String>,
    /// Request body as JSON.
    #[arg(long, value_parser = parse_json_value)]
    pub body: Option<serde_json::Value>,
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
            path_type: Path::ViewerSend,
            path: args.path.ok_or_else(|| {
                crate::cli::command::FromArgsError::path_parse(
                    "path",
                    "--path is required".to_string(),
                )
            })?,
            body: args.body.ok_or_else(|| {
                crate::cli::command::FromArgsError::path_parse(
                    "body",
                    "--body is required".to_string(),
                )
            })?,
            base: args.base.into(),
        })
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
