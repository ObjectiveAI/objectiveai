//! `functions execute swiss-system` — async handler stub.

use crate::cli::command::CommandRequest;
use crate::functions::expression::InputValue;
use super::{FunctionArgs, FunctionSpec, ProfileArgs, ProfileSpec};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.execute.swiss_system.Request")]
pub struct Request {
    pub path_type: Path,
    pub function: FunctionSpec,
    pub profile: ProfileSpec,
    pub input: RequestInput,
    pub continuation: Option<String>,
    pub split: bool,
    pub invert: bool,
    pub pool: Option<usize>,
    pub rounds: Option<usize>,
    pub dangerous_advanced: Option<RequestDangerousAdvanced>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.execute.swiss_system.Path")]
pub enum Path {
    #[serde(rename = "functions/execute/swiss_system")]
    FunctionsExecuteSwissSystem,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.execute.swiss_system.RequestInput")]
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
#[schemars(rename = "cli.command.functions.execute.swiss_system.RequestDangerousAdvanced")]
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.functions.execute.swiss_system.ResponseItem")]
pub enum ResponseItem {
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
/// | --input-python-file`. See
/// `super::standard::InputArgs` for the group-id rationale.
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
    /// How many vector responses per execution.
    #[arg(long)]
    pub pool: Option<usize>,
    /// How many sequential rounds of comparison.
    #[arg(long)]
    pub rounds: Option<usize>,
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
        Ok(Self { path_type: Path::FunctionsExecuteSwissSystem,
            function,
            profile,
            input,
            continuation: args.continuation,
            split: args.split,
            invert: args.invert,
            pool: args.pool,
            rounds: args.rounds,
            dangerous_advanced,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute_streaming<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<E::Stream<ResponseItem>, E::Error> {
    request.base.clear_transform();
    let mut advanced = request.dangerous_advanced.unwrap_or_default();
    advanced.stream = Some(true);
    request.dangerous_advanced = Some(advanced);
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_streaming_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    transform: crate::cli::command::Transform,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<E::Stream<serde_json::Value>, E::Error> {
    request.base.set_transform(transform);
    let mut advanced = request.dangerous_advanced.unwrap_or_default();
    advanced.stream = Some(true);
    request.dangerous_advanced = Some(advanced);
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<Response, E::Error> {
    request.base.clear_transform();
    if let Some(advanced) = request.dangerous_advanced.as_mut() {
        advanced.stream = None;
    }
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
    if let Some(advanced) = request.dangerous_advanced.as_mut() {
        advanced.stream = None;
    }
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;

pub mod response_schema;

/// One `/listen` broadcast run of `functions execute swiss_system`: the actual
/// [`Request`], the producer's
/// [`AgentArguments`](crate::cli::command::AgentArguments), and the
/// response — in whichever of the leaf's two forms the request
/// selected. See [`crate::cli::websocket_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: ListenerExecutionResponse,
}

/// The dual-form leaf's response: unary by default, streaming when
/// the request set `dangerous_advanced.stream: true`.
#[cfg(feature = "cli-listener")]
pub enum ListenerExecutionResponse {
    Unary(crate::cli::websocket_listener::UnaryResponse<Response>),
    Streaming(crate::cli::websocket_listener::ResponseItemStream<ResponseItem>),
}
