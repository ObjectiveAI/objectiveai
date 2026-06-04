//! `plugins run` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.Request")]
pub struct Request {
    pub path: Path,
    pub name: String,
    pub args: Vec<String>,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.plugins.Path")]
pub enum Path {
    #[serde(rename = "plugins/run")]
    PluginsRun,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["plugins".to_string(), "run".to_string()];
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv.push(self.name.clone());
        argv.extend(self.args.iter().cloned());
        argv
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.plugins.ResponseItem")]
pub enum ResponseItem {
    Mcp(Mcp),
    // `cli::Error` already carries `type:"error"`. Placement above
    // `Notification` is load-bearing: serde untagged tries variants
    // in source order, so a `cli::Error`-shaped JSON must match
    // `Error` before falling through to the catch-all.
    Error(crate::cli::Error),
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
#[schemars(rename = "cli.command.plugins.Mcp")]
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
#[schemars(rename = "cli.command.plugins.McpType")]
pub enum McpType {
    Mcp,
}

#[derive(clap::Args)]
pub struct Args {
    /// Plugin/tool name.
    pub name: String,
    /// Arguments passed through to the invoked binary.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
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
        Ok(Self { path: Path::PluginsRun,
            name: args.name,
            args: args.args,
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

pub mod request_schema {
    use crate::cli::command::CommandRequest;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    pub struct Request {
        pub path: Path,
        pub jq: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    pub enum Path {
        #[serde(rename = "plugins/run/request_schema")]
        PluginsRunRequestSchema,
    }
    #[derive(clap::Args)]
    pub struct Args {
        #[arg(long)]
        pub jq: Option<String>,
    }


    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            let mut argv: Vec<String> = vec!["plugins", "run", "request-schema"].into_iter().map(String::from).collect();
            if let Some(jq) = &self.jq {
                argv.push("--jq".to_string());
                argv.push(jq.clone());
            }
            argv
        }
    }

    pub type Response = schemars::Schema;

    impl TryFrom<Args> for Request {
        type Error = crate::cli::command::FromArgsError;
        fn try_from(args: Args) -> Result<Self, Self::Error> {
            Ok(Self { path: Path::PluginsRunRequestSchema, jq: args.jq })
        }
    }

    #[cfg(feature = "cli-executor")]
    pub async fn execute<E: crate::cli::command::CommandExecutor>(
        executor: &E,
        mut request: Request,
    
        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<Response, E::Error> {
        request.jq = None;
        executor.execute_one(request, agent_arguments).await
    }

    #[cfg(feature = "cli-executor")]
    pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
        executor: &E,
        mut request: Request,
        jq: String,
    
        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<serde_json::Value, E::Error> {
        request.jq = Some(jq);
        executor.execute_one(request, agent_arguments).await
    }
}


pub mod response_schema {
    use crate::cli::command::CommandRequest;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    pub struct Request {
        pub path: Path,
        pub jq: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    pub enum Path {
        #[serde(rename = "plugins/run/response_schema")]
        PluginsRunResponseSchema,
    }
    #[derive(clap::Args)]
    pub struct Args {
        #[arg(long)]
        pub jq: Option<String>,
    }


    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            let mut argv: Vec<String> = vec!["plugins", "run", "response-schema"].into_iter().map(String::from).collect();
            if let Some(jq) = &self.jq {
                argv.push("--jq".to_string());
                argv.push(jq.clone());
            }
            argv
        }
    }

    pub type Response = schemars::Schema;

    impl TryFrom<Args> for Request {
        type Error = crate::cli::command::FromArgsError;
        fn try_from(args: Args) -> Result<Self, Self::Error> {
            Ok(Self { path: Path::PluginsRunResponseSchema, jq: args.jq })
        }
    }

    #[cfg(feature = "cli-executor")]
    pub async fn execute<E: crate::cli::command::CommandExecutor>(
        executor: &E,
        mut request: Request,
    
        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<Response, E::Error> {
        request.jq = None;
        executor.execute_one(request, agent_arguments).await
    }

    #[cfg(feature = "cli-executor")]
    pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
        executor: &E,
        mut request: Request,
        jq: String,
    
        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<serde_json::Value, E::Error> {
        request.jq = Some(jq);
        executor.execute_one(request, agent_arguments).await
    }
}
