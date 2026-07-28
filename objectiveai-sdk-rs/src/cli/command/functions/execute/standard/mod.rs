//! `functions execute standard` — async handler stub.

use crate::agent::completions::message::Message;  // unused placeholder to keep imports tidy
use crate::cli::command::CommandRequest;
use crate::functions::expression::InputValue;
use super::{FunctionArgs, FunctionSpec, ProfileArgs, ProfileSpec};

#[allow(dead_code)]
type _UnusedMessage = Message;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.execute.standard.Request")]
pub struct Request {
    pub path_type: Path,
    pub function: FunctionSpec,
    pub profile: ProfileSpec,
    pub input: RequestInput,
    pub continuation: Option<String>,
    pub split: bool,
    pub invert: bool,
    pub dangerous_advanced: Option<RequestDangerousAdvanced>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.execute.standard.Path")]
pub enum Path {
    #[serde(rename = "functions/execute/standard")]
    FunctionsExecuteStandard,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.execute.standard.RequestInput")]
pub enum RequestInput {
    #[schemars(title = "Inline")]
    Inline(InputValue),
    #[schemars(title = "File")]
    File(std::path::PathBuf),
    #[schemars(title = "PythonInline")]
    PythonInline(String),
    #[schemars(title = "PythonFile")]
    PythonFile(std::path::PathBuf),
}

impl RequestInput {
    fn push_flags(&self, out: &mut Vec<String>) {
        match self {
            RequestInput::Inline(v) => {
                out.push("--input-inline".to_string());
                out.push(serde_json::to_string(v).expect("input serializes"));
            }
            RequestInput::File(p) => {
                out.push("--input-file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
            RequestInput::PythonInline(code) => {
                out.push("--input-python-inline".to_string());
                out.push(code.clone());
            }
            RequestInput::PythonFile(p) => {
                out.push("--input-python-file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
        }
    }
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.execute.standard.RequestDangerousAdvanced")]
pub struct RequestDangerousAdvanced {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream: Option<bool>,
    /// Deterministic seed for downstream mock agents. Forwarded
    /// to every per-task `AgentCompletionCreateParams.seed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub seed: Option<i64>,
}

/// A unique agent instance participating in this execution, announced
/// exactly once — right after its instance lock is acquired (the same
/// moment `agents spawn` announces its own hierarchy). The constant
/// `type:"agent_instance_hierarchy"` discriminator disambiguates this
/// variant inside the untagged [`ResponseItem`] union, mirroring
/// `type:"mcp"` on `plugins run`'s `Mcp`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.execute.standard.AgentInstanceHierarchy")]
pub struct AgentInstanceHierarchy {
    pub r#type: AgentInstanceHierarchyType,
    pub agent_instance_hierarchy: String,
}

/// Single-variant discriminator for [`AgentInstanceHierarchy`]'s
/// `type` field. Always `"agent_instance_hierarchy"` on the wire.
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
#[schemars(rename = "cli.command.functions.execute.standard.AgentInstanceHierarchyType")]
pub enum AgentInstanceHierarchyType {
    AgentInstanceHierarchy,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.functions.execute.standard.ResponseItem")]
pub enum ResponseItem {
    // Placement above `Chunk` is load-bearing: serde untagged tries
    // variants in source order, and the constant discriminator must
    // win before the all-optional chunk object could absorb it.
    #[schemars(title = "AgentInstanceHierarchy")]
    AgentInstanceHierarchy(AgentInstanceHierarchy),
    #[schemars(title = "Chunk")]
    Chunk(crate::functions::executions::response::streaming::FunctionExecutionChunk),
    #[schemars(title = "Id")]
    Id(String),
}

/// Non-chunk variant of [`ResponseItem`]. Returned by the unary `execute`
/// path (with `dangerous_advanced.stream` cleared) when the cli emits a
/// single bare id string.
pub type Response = String;

/// Exactly-one-of `--input-inline | --input-file | --input-python-inline
/// | --input-python-file`. Scoped to its own `#[group]` annotation on a
/// dedicated sub-struct so the `required = true, multiple = false`
/// enforcement only applies to these fields — hoisting the annotation
/// to the outer [`Args`] would pull every field into the "input_group"
/// group via clap derive's default-group rule. Mirrors the
/// `super::{FunctionArgs, ProfileArgs}` pattern.
#[derive(clap::Args)]
#[group(id = "input_group", required = true, multiple = false)]
pub struct InputArgs {
    /// Inline JSON input value.
    #[arg(long, group = "input_group")]
    pub input_inline: Option<String>,
    /// Path to a JSON file containing the input value.
    #[arg(long, group = "input_group")]
    pub input_file: Option<std::path::PathBuf>,
    /// Inline Python that produces the input value.
    #[arg(long, group = "input_group")]
    pub input_python_inline: Option<String>,
    /// Path to a Python file that produces the input value.
    #[arg(long, group = "input_group")]
    pub input_python_file: Option<std::path::PathBuf>,
}

#[derive(clap::Args)]
pub struct Args {
    /// Exactly one of `--function`, `--function-inline`,
    /// `--function-file`, `--function-python-inline`,
    /// `--function-python-file`.
    #[command(flatten)]
    pub function: FunctionArgs,
    /// Exactly one of `--profile`, `--profile-inline`,
    /// `--profile-file`, `--profile-python-inline`,
    /// `--profile-python-file`.
    #[command(flatten)]
    pub profile: ProfileArgs,
    /// Exactly one of `--input-inline`, `--input-file`,
    /// `--input-python-inline`, `--input-python-file`.
    #[command(flatten)]
    pub input: InputArgs,
    /// Continuation token from a previous response.
    #[arg(long)]
    pub continuation: Option<String>,
    /// Treat input as an array and execute once per element.
    #[arg(long)]
    pub split: bool,
    /// Invert outputs after expressions evaluate.
    #[arg(long)]
    pub invert: bool,
    /// Advanced opt-in flags as inline JSON.
    #[arg(long)]
    pub dangerous_advanced: Option<String>,
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
        let function = FunctionSpec::try_from(args.function)?;
        let profile = ProfileSpec::try_from(args.profile)?;
        let input = if let Some(s) = args.input.input_inline {
            let mut de = serde_json::Deserializer::from_str(&s);
            let v = serde_path_to_error::deserialize(&mut de)
                .map_err(|e| crate::cli::command::FromArgsError::json("input_inline", e))?;
            RequestInput::Inline(v)
        } else if let Some(p) = args.input.input_file {
            RequestInput::File(p)
        } else if let Some(s) = args.input.input_python_inline {
            RequestInput::PythonInline(s)
        } else {
            RequestInput::PythonFile(args.input.input_python_file.unwrap())
        };
        let dangerous_advanced: Option<RequestDangerousAdvanced> =
            if let Some(s) = args.dangerous_advanced {
                let mut de = serde_json::Deserializer::from_str(&s);
                let v = serde_path_to_error::deserialize(&mut de)
                    .map_err(|e| crate::cli::command::FromArgsError::json("dangerous_advanced", e))?;
                Some(v)
            } else {
                None
            };
        Ok(Self { path_type: Path::FunctionsExecuteStandard,
            function,
            profile,
            input,
            continuation: args.continuation,
            split: args.split,
            invert: args.invert,
            dangerous_advanced,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute_streaming<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,

        identity: Option<&crate::identity::Identity>,
    ) -> Result<E::Stream<ResponseItem>, E::Error> {
    request.base.clear_transform();
    let mut advanced = request.dangerous_advanced.unwrap_or_default();
    advanced.stream = Some(true);
    request.dangerous_advanced = Some(advanced);
    executor.execute(request, identity).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_streaming_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    transform: crate::cli::command::Transform,

        identity: Option<&crate::identity::Identity>,
    ) -> Result<E::Stream<serde_json::Value>, E::Error> {
    request.base.set_transform(transform);
    let mut advanced = request.dangerous_advanced.unwrap_or_default();
    advanced.stream = Some(true);
    request.dangerous_advanced = Some(advanced);
    executor.execute(request, identity).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,

        identity: Option<&crate::identity::Identity>,
    ) -> Result<Response, E::Error> {
    request.base.clear_transform();
    if let Some(advanced) = request.dangerous_advanced.as_mut() {
        advanced.stream = None;
    }
    executor.execute_one(request, identity).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    transform: crate::cli::command::Transform,

        identity: Option<&crate::identity::Identity>,
    ) -> Result<serde_json::Value, E::Error> {
    request.base.set_transform(transform);
    if let Some(advanced) = request.dangerous_advanced.as_mut() {
        advanced.stream = None;
    }
    executor.execute_one(request, identity).await
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;

pub mod response_schema;

/// One `/listen` broadcast run of `functions execute standard` in its unary
/// form (the plain `execute`): the actual [`Request`], the
/// producer's
/// [`Identity`](crate::identity::Identity), and the
/// unary response future. See [`crate::daemon::command_listener`].
#[cfg(all(feature = "cli", feature = "daemon"))]
pub struct ListenerExecution {
    pub request: Request,
    pub identity: crate::identity::Identity,
    pub response: crate::daemon::command_listener::UnaryResponse<Response>,
}

/// One `/listen` broadcast run of `functions execute standard` in its
/// streaming form (`execute_streaming` — the request set
/// `dangerous_advanced.stream: true`): the actual [`Request`], the
/// producer's
/// [`Identity`](crate::identity::Identity), and the
/// response-item stream. See [`crate::daemon::command_listener`].
#[cfg(all(feature = "cli", feature = "daemon"))]
pub struct ListenerExecutionStreaming {
    pub request: Request,
    pub identity: crate::identity::Identity,
    pub response: crate::daemon::command_listener::ResponseItemStream<ResponseItem>,
}

/// This leaf's multiple listener executions — one variant per
/// execute fn (`Execution` for the plain `execute`, `Streaming`
/// for `execute_streaming`), discriminated per request off
/// `dangerous_advanced.stream`. The branch enum's single variant
/// for this leaf wraps this.
#[cfg(all(feature = "cli", feature = "daemon"))]
pub enum ListenerExecutionVariant {
    Execution(ListenerExecution),
    Streaming(ListenerExecutionStreaming),
}
