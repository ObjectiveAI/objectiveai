//! `config functions profiles pairs favorites get` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.config.functions.profiles.pairs.favorites.get.Request")]
pub struct Request {
    pub path_type: Path,
    pub scope: crate::cli::command::config::GetScope,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.config.functions.profiles.pairs.favorites.get.Path")]
pub enum Path {
    #[serde(rename = "config/functions/profiles/pairs/favorites/get")]
    ConfigFunctionsProfilesPairsFavoritesGet,
}
impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["config".to_string(), "functions".to_string(), "profiles".to_string(), "pairs".to_string(), "favorites".to_string(), "get".to_string()];
        argv.push(match self.scope {
            crate::cli::command::config::GetScope::Global => "--global".to_string(),
            crate::cli::command::config::GetScope::State => "--state".to_string(),
            crate::cli::command::config::GetScope::Final => "--final".to_string(),
        });
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.config.functions.profiles.pairs.favorites.get.ResponseItem")]
pub struct ResponseItem {
    pub name: String,
    pub function: crate::RemotePathCommitOptional,
    pub profile: crate::RemotePathCommitOptional,
    pub note: String,
}

#[derive(clap::Args)]
pub struct Args {
    /// Read the global config layer.
    #[arg(long)]
    pub global: bool,
    /// Read the state config layer.
    #[arg(long)]
    pub state: bool,
    /// Read the final merged config view.
    #[arg(long)]
    pub r#final: bool,
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
        let scope = match (args.global, args.state, args.r#final) {
            (true, false, false) => crate::cli::command::config::GetScope::Global,
            (false, true, false) => crate::cli::command::config::GetScope::State,
            (false, false, true) => crate::cli::command::config::GetScope::Final,
            _ => {
                return Err(crate::cli::command::FromArgsError {
                    field: "scope",
                    source: crate::cli::command::FromArgsErrorSource::Plain(
                        "exactly one of --global, --state, --final is required".to_string(),
                    ),
                });
            }
        };
        Ok(Self { path_type: Path::ConfigFunctionsProfilesPairsFavoritesGet, scope, jq: args.jq })
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

pub mod request_schema;

pub mod response_schema;
