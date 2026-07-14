//! `config db user set` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.db.config.user.set.Request")]
pub struct Request {
    pub path_type: Path,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
    pub scope: crate::cli::command::SetScope,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.db.config.user.set.Path")]
pub enum Path {
    #[serde(rename = "db/config/user/set")]
    DbConfigUserSet,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

pub type Response = crate::cli::command::Ok;

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("value_required").required(true).args(["value"])))]
pub struct Args {
    /// Mutate the global config layer.
    #[arg(long)]
    pub global: bool,
    /// Mutate the state config layer.
    #[arg(long)]
    pub state: bool,
    #[command(flatten)]
    pub base: crate::cli::command::RequestBaseArgs,
    /// New value.
    #[arg(long)]
    pub value: Option<String>,
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
        let scope = match (args.global, args.state) {
            (true, false) => crate::cli::command::SetScope::Global,
            (false, true) => crate::cli::command::SetScope::State,
            _ => {
                return Err(crate::cli::command::FromArgsError {
                    field: "scope",
                    source: crate::cli::command::FromArgsErrorSource::Plain(
                        "exactly one of --global, --state is required".to_string(),
                    ),
                });
            }
        };
        Ok(Self {
            base: args.base.into(), path_type: Path::DbConfigUserSet,
            scope,
            value: args.value.ok_or_else(|| {
                crate::cli::command::FromArgsError::path_parse(
                    "value",
                    "--value is required".to_string(),
                )
            })?,
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<Response, E::Error> {
    executor.execute_one(request, agent_arguments).await
}

pub mod request_schema;

pub mod response_schema;

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    _transform: crate::cli::command::Transform,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<serde_json::Value, E::Error> {
    let resp: Response = executor.execute_one(request, agent_arguments).await?;
    Ok(serde_json::to_value(resp).expect("Response serializes"))
}

/// One `/listen` broadcast run of `db config user set`: the actual
/// [`Request`], the producer's
/// [`AgentArguments`](crate::cli::command::AgentArguments), and the
/// unary response future. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::broadcast_listener::UnaryResponse<Response>,
}
