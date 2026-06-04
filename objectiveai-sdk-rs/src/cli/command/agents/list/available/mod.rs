//! `agents list available` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.list.available.Request")]
pub struct Request {
    pub path_type: Path,
    pub source: RequestSource,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.list.available.Path")]
pub enum Path {
    #[serde(rename = "agents/list/available")]
    AgentsListAvailable,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, clap::ValueEnum)]
#[clap(rename_all = "kebab-case")]
#[schemars(rename = "cli.command.agents.list.available.RequestSource")]
pub enum RequestSource {
    Filesystem,
    Favorites,
    Objectiveai,
    Mock,
    All,
}

impl RequestSource {
    fn as_str(&self) -> &'static str {
        match self {
            RequestSource::Filesystem => "filesystem",
            RequestSource::Favorites => "favorites",
            RequestSource::Objectiveai => "objectiveai",
            RequestSource::Mock => "mock",
            RequestSource::All => "all",
        }
    }
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "list".to_string(),
            "available".to_string(),
            "--source".to_string(),
            self.source.as_str().to_string(),
        ];
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.list.available.ResponseFavorite")]
pub struct ResponseFavorite {
    pub name: String,
    #[serde(flatten)]
    pub path: crate::RemotePathCommitOptional,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.list.available.ResponseItem")]
pub enum ResponseItem {
    #[schemars(title = "Favorite")]
    Favorite(ResponseFavorite),
    #[schemars(title = "Item")]
    Item(crate::RemotePath),
}

#[derive(clap::Args)]
pub struct Args {
    /// Which source to list from.
    #[arg(long, value_enum)]
    pub source: RequestSource,
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
        Ok(Self { path_type: Path::AgentsListAvailable,
            source: args.source,
            jq: args.jq,
        })
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
