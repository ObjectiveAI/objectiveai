//! `swarms publish` — async handler stub.

use crate::swarm::RemoteSwarmBase;
use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.swarms.publish.Request")]
pub struct Request {
    pub path_type: Path,
    pub repository: String,
    pub body: RequestBody,
    pub message: RequestPublishMessage,
    pub overwrite: bool,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.swarms.publish.Path")]
pub enum Path {
    #[serde(rename = "swarms/publish")]
    SwarmsPublish,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.swarms.publish.RequestBody")]
pub enum RequestBody {
    #[schemars(title = "Inline")]
    Inline(RemoteSwarmBase),
    #[schemars(title = "File")]
    File(std::path::PathBuf),
    #[schemars(title = "PythonInline")]
    PythonInline(String),
    #[schemars(title = "PythonFile")]
    PythonFile(std::path::PathBuf),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.swarms.publish.RequestPublishMessage")]
pub enum RequestPublishMessage {
    #[schemars(title = "Inline")]
    Inline(String),
    #[schemars(title = "File")]
    File(std::path::PathBuf),
}

impl RequestBody {
    fn push_flags(&self, out: &mut Vec<String>) {
        match self {
            RequestBody::Inline(v) => {
                out.push("--body-inline".to_string());
                out.push(serde_json::to_string(v).expect("body serializes"));
            }
            RequestBody::File(p) => {
                out.push("--body-file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
            RequestBody::PythonInline(code) => {
                out.push("--body-python-inline".to_string());
                out.push(code.clone());
            }
            RequestBody::PythonFile(p) => {
                out.push("--body-python-file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
        }
    }
}

impl RequestPublishMessage {
    fn push_flags(&self, out: &mut Vec<String>) {
        match self {
            RequestPublishMessage::Inline(s) => {
                out.push("--message-inline".to_string());
                out.push(s.clone());
            }
            RequestPublishMessage::File(p) => {
                out.push("--message-file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
        }
    }
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "swarms".to_string(),
            "publish".to_string(),
            "--repository".to_string(),
            self.repository.clone(),
        ];
        self.body.push_flags(&mut argv);
        self.message.push_flags(&mut argv);
        if self.overwrite {
            argv.push("--overwrite".to_string());
        }
        self.base.push_flags(&mut argv);
        argv
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.swarms.publish.Response")]
pub struct Response {
    pub sha: String,
}

#[derive(clap::Args)]
#[group(id = "body", required = true, multiple = false)]
#[group(id = "message", required = true, multiple = false)]
pub struct Args {
    /// Repository name.
    #[arg(long)]
    pub repository: String,
    /// Inline JSON body.
    #[arg(long, group = "body")]
    pub body_inline: Option<String>,
    /// Path to a JSON file.
    #[arg(long, group = "body")]
    pub body_file: Option<std::path::PathBuf>,
    /// Inline Python that produces the JSON body.
    #[arg(long, group = "body")]
    pub body_python_inline: Option<String>,
    /// Path to a Python file that produces the JSON body.
    #[arg(long, group = "body")]
    pub body_python_file: Option<std::path::PathBuf>,
    /// Inline commit message.
    #[arg(long, group = "message")]
    pub message_inline: Option<String>,
    /// Path to a file containing the commit message.
    #[arg(long, group = "message")]
    pub message_file: Option<std::path::PathBuf>,
    /// Overwrite if the entry already exists.
    #[arg(long)]
    pub overwrite: bool,
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
        let body = if let Some(s) = args.body_inline {
            let mut de = serde_json::Deserializer::from_str(&s);
            let v = serde_path_to_error::deserialize(&mut de).map_err(|source| {
                crate::cli::command::FromArgsError {
                    field: "body_inline",
                    source: source.into(),
                }
            })?;
            RequestBody::Inline(v)
        } else if let Some(p) = args.body_file {
            RequestBody::File(p)
        } else if let Some(s) = args.body_python_inline {
            RequestBody::PythonInline(s)
        } else {
            RequestBody::PythonFile(args.body_python_file.unwrap())
        };
        let message = if let Some(s) = args.message_inline {
            RequestPublishMessage::Inline(s)
        } else {
            RequestPublishMessage::File(args.message_file.unwrap())
        };
        Ok(Self { path_type: Path::SwarmsPublish,
            repository: args.repository,
            body,
            message,
            overwrite: args.overwrite,
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
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;


pub mod response_schema;
