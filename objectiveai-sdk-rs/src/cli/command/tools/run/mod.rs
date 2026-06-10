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
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tools.run.Path")]
pub enum Path {
    #[serde(rename = "tools/run")]
    ToolsRun,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["tools".to_string(), "run".to_string()];
        argv.push("--owner".to_string());
        argv.push(self.owner.clone());
        argv.push("--name".to_string());
        argv.push(self.name.clone());
        argv.push("--version".to_string());
        argv.push(self.version.clone());
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        if !self.args.is_empty() {
            argv.push("--args".to_string());
            argv.push(serde_json::to_string(&self.args).expect("Vec<String> serializes"));
        }
        argv
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
pub struct Args {
    /// Tool owner (GitHub `<owner>` segment). Required.
    #[arg(long)]
    pub owner: String,
    /// Tool name (repository segment). Required.
    #[arg(long)]
    pub name: String,
    /// Tool version. Required.
    #[arg(long)]
    pub version: String,
    /// Arguments appended to the tool's exec vector, as a JSON array
    /// of strings (e.g. `--args '["--flag","value"]'`).
    #[arg(long)]
    pub args: Option<String>,
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
            owner: args.owner,
            name: args.name,
            version: args.version,
            args: parsed_args,
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
