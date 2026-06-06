//! `config swarms favorites add` — async handler stub.

use crate::RemotePathCommitOptional;
use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.config.swarms.favorites.add.Request")]
pub struct Request {
    pub path_type: Path,
    pub name: String,
    pub path: RemotePathCommitOptional,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.config.swarms.favorites.add.Path")]
pub enum Path {
    #[serde(rename = "config/swarms/favorites/add")]
    ConfigSwarmsFavoritesAdd,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        vec![
            "config".to_string(),
            "swarms".to_string(),
            "favorites".to_string(),
            "add".to_string(),
            "--name".to_string(),
            self.name.clone(),
            "--path".to_string(),
            crate::cli::command::remote_path_to_arg_string(&self.path),
            "--note".to_string(),
            self.note.clone(),
        ]
    }
}

pub type Response = crate::cli::command::Ok;

#[derive(clap::Args)]
pub struct Args {
    /// Favorite name.
    #[arg(long)]
    pub name: String,
    /// Remote-path string (docker-style: `remote=<github|filesystem|mock>,owner=…,repository=…[,commit=…]`).
    #[arg(long)]
    pub path: String,
    /// Free-text note describing the favorite.
    #[arg(long)]
    pub note: String,
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
            .parse::<RemotePathCommitOptional>()
            .map_err(|msg| crate::cli::command::FromArgsError::path_parse("path", msg))?;
        Ok(Self {
            path_type: Path::ConfigSwarmsFavoritesAdd,
            name: args.name,
            path,
            note: args.note,
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
pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    _jq: String,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<serde_json::Value, E::Error> {
    let resp: Response = executor.execute_one(request, agent_arguments).await?;
    Ok(serde_json::to_value(resp).expect("Response serializes"))
}
