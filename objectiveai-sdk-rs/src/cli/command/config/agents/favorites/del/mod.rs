//! `config agents favorites del` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.config.agents.favorites.del.Request")]
pub struct Request {
    pub path_type: Path,
    pub scope: crate::cli::command::config::SetScope,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.config.agents.favorites.del.Path")]
pub enum Path {
    #[serde(rename = "config/agents/favorites/del")]
    ConfigAgentsFavoritesDel,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "config".to_string(),
            "agents".to_string(),
            "favorites".to_string(),
            "del".to_string(),
            self.name.clone(),
        ];
        argv.push(match self.scope {
            crate::cli::command::config::SetScope::Global => "--global".to_string(),
            crate::cli::command::config::SetScope::State => "--state".to_string(),
        });
        argv
    }
}

pub type Response = crate::cli::command::Ok;

#[derive(clap::Args)]
pub struct Args {
    /// Mutate the global config layer.
    #[arg(long)]
    pub global: bool,
    /// Mutate the state config layer.
    #[arg(long)]
    pub state: bool,
    /// Favorite name.
    pub name: String,
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
            (true, false) => crate::cli::command::config::SetScope::Global,
            (false, true) => crate::cli::command::config::SetScope::State,
            _ => {
                return Err(crate::cli::command::FromArgsError {
                    field: "scope",
                    source: crate::cli::command::FromArgsErrorSource::Plain(
                        "exactly one of --global, --state is required".to_string(),
                    ),
                });
            }
        };
        Ok(Self { path_type: Path::ConfigAgentsFavoritesDel, scope, name: args.name })
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
