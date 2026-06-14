//! `tools get` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tools.get.Request")]
pub struct Request {
    pub path_type: Path,
    pub owner: String,
    pub name: String,
    pub version: String,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tools.get.Path")]
pub enum Path {
    #[serde(rename = "tools/get")]
    ToolsGet,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "tools".to_string(),
            "get".to_string(),
            "--owner".to_string(),
            self.owner.clone(),
            "--name".to_string(),
            self.name.clone(),
            "--version".to_string(),
            self.version.clone(),
        ];
        self.base.push_flags(&mut argv);
        argv
    }

    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// Per-OS exec command for a tool. The current platform's vector is
/// the program plus its leading arguments; the caller's `--args` are
/// appended, and the result runs with CWD = the tool's version folder
/// (where `objectiveai.json` lives).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tools.get.Exec")]
pub struct Exec {
    pub windows: Vec<String>,
    pub linux: Vec<String>,
    pub macos: Vec<String>,
}

impl Exec {
    pub fn is_empty(&self) -> bool {
        self.windows.is_empty() && self.linux.is_empty() && self.macos.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tools.get.ResponseManifest")]
pub struct ResponseManifest {
    pub name: String,
    pub description: String,
    pub version: String,
    pub owner: String,
    pub exec: Exec,
    pub source: String,
}

impl ResponseManifest {
    /// LLM-visible MCP tool name for this tool:
    /// `tool_{owner}_{name}_{version}`, with every `.` in the version
    /// substituted to `-` so the result stays within the Anthropic
    /// tool-name regex (`^[a-zA-Z0-9_-]{1,128}$`). `objectiveai-mcp`
    /// advertises each tool under this name.
    pub fn tool_name(&self) -> String {
        format!(
            "tool_{}_{}_{}",
            self.owner,
            self.name,
            self.version.replace('.', "-")
        )
    }
}

pub type Response = Option<ResponseManifest>;

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
            path_type: Path::ToolsGet,
            owner: args.owner,
            name: args.name,
            version: args.version,
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

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseManifest {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;


pub mod response_schema;
