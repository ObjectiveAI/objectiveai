//! `plugins run` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.run.Request")]
pub struct Request {
    pub path_type: Path,
    pub owner: String,
    pub name: String,
    pub version: String,
    pub args: Vec<String>,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.run.Path")]
pub enum Path {
    #[serde(rename = "plugins/run")]
    PluginsRun,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["plugins".to_string(), "run".to_string()];
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
#[schemars(rename = "cli.command.plugins.run.ResponseItem")]
pub enum ResponseItem {
    #[schemars(title = "Mcp")]
    Mcp(Mcp),
    // `cli::Error` already carries `type:"error"`. Placement above
    // `Notification` is load-bearing: serde untagged tries variants
    // in source order, so a `cli::Error`-shaped JSON must match
    // `Error` before falling through to the catch-all.
    #[schemars(title = "Error")]
    Error(crate::cli::Error),
    #[schemars(title = "Notification")]
    Notification(serde_json::Value),
}

/// Plugin announces a running MCP server URL. The host routes this
/// through the standard plugin-notification pipeline and dials the
/// URL the same way it would for an entry in the plugin's manifest
/// `mcp_servers` — runtime announcements are functionally identical
/// to manifest-time declarations.
///
/// The constant `type:"mcp"` discriminator disambiguates this
/// variant from the rest of the untagged [`ResponseItem`] /
/// [`crate::cli::plugins::Output`] catch-all, mirroring the
/// `type:"error"` discriminator on [`crate::cli::Error`].
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.run.Mcp")]
pub struct Mcp {
    pub r#type: McpType,
    pub url: String,
}

/// Single-variant discriminator for [`Mcp`]'s `type` field. Always
/// `"mcp"` on the wire.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "cli.command.plugins.run.McpType")]
pub enum McpType {
    Mcp,
}

#[derive(clap::Args)]
pub struct Args {
    /// Plugin owner (GitHub `<owner>` segment). Required.
    #[arg(long)]
    pub owner: String,
    /// Plugin name (repository segment). Required.
    #[arg(long)]
    pub name: String,
    /// Plugin version. Required.
    #[arg(long)]
    pub version: String,
    /// Arguments passed through to the invoked binary, as a JSON array
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
            path_type: Path::PluginsRun,
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
            ResponseItem::Mcp(m) => {
                McpResponseItem::JSONL(serde_json::to_value(m).unwrap())
            }
            ResponseItem::Error(e) => e.into_mcp(),
            ResponseItem::Notification(value) => {
                // String + data URL → media via RichContentPart::from_blob.
                // Anything else (and strings that aren't data URLs) →
                // JSONL passthrough.
                if let serde_json::Value::String(s) = &value
                    && let Some((mime, payload)) = crate::data_url::parse_data_url(s)
                {
                    let part = RichContentPart::from_blob(
                        mime,
                        payload.to_string(),
                        None,
                    );
                    return McpResponseItem::Media(part.into());
                }
                McpResponseItem::JSONL(value)
            }
        }
    }
}

pub mod request_schema;


pub mod response_schema;
