//! `tools run` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tools.run.Request")]
pub struct Request {
    pub path_type: Path,
    pub owner: String,
    pub name: String,
    pub version: String,
    pub args: Vec<String>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tools.run.Path")]
pub enum Path {
    #[serde(rename = "tools/run")]
    ToolsRun,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.tools.run.ResponseItem")]
pub enum ResponseItem {
    #[schemars(title = "Stdout")]
    Stdout(String),
    #[schemars(title = "Stderr")]
    Stderr(crate::cli::Error),
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("owner_required").required(true).args(["owner"])))]
#[command(group(clap::ArgGroup::new("name_required").required(true).args(["name"])))]
#[command(group(clap::ArgGroup::new("version_required").required(true).args(["version"])))]
pub struct Args {
    /// Tool owner (GitHub `<owner>` segment). Required.
    #[arg(long)]
    pub owner: Option<String>,
    /// Tool name (repository segment). Required.
    #[arg(long)]
    pub name: Option<String>,
    /// Tool version. Required.
    #[arg(long)]
    pub version: Option<String>,
    /// Arguments appended to the tool's exec vector, as a JSON array
    /// of strings (e.g. `--args '["--flag","value"]'`).
    #[arg(long)]
    pub args: Option<String>,
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
        let parsed_args: Vec<String> = match args.args {
            Some(s) => {
                let mut de = serde_json::Deserializer::from_str(&s);
                serde_path_to_error::deserialize(&mut de).map_err(|source| {
                    crate::cli::command::FromArgsError {
                        field: "args",
                        source: source.into(),
                    }
                })?
            }
            None => Vec::new(),
        };
        Ok(Self {
            path_type: Path::ToolsRun,
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
            args: parsed_args,
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
        use crate::agent::completions::message::RichContentPart;
        use crate::cli::command::McpResponseItem;
        match self {
            ResponseItem::Stdout(s) => {
                // Stdout line that happens to be a `data:<mime>;base64,...`
                // URL gets upgraded to a typed media block; otherwise
                // it rides through as a bare `Value::String`.
                if let Some((mime, payload)) = crate::data_url::parse_data_url(&s) {
                    let part = RichContentPart::from_blob(
                        mime,
                        payload.to_string(),
                        None,
                    );
                    return McpResponseItem::Media(part.into());
                }
                McpResponseItem::JSONL(serde_json::Value::String(s))
            }
            ResponseItem::Stderr(e) => e.into_mcp(),
        }
    }
}

pub mod request_schema;

pub mod response_schema;

/// One `/listen` broadcast run of `tools run`: the actual
/// [`Request`], the producer's
/// [`AgentArguments`](crate::cli::command::AgentArguments), and the
/// response-item stream. See [`crate::cli::websocket_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::websocket_listener::ResponseItemStream<ResponseItem>,
}
