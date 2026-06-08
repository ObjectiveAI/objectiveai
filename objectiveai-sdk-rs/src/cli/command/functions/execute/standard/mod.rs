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
    pub retry_token: Option<String>,
    pub seed: Option<i64>,
    pub split: bool,
    pub invert: bool,
    pub dangerous_advanced: Option<RequestDangerousAdvanced>,
    pub jq: Option<String>,
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
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "functions".to_string(),
            "execute".to_string(),
            "standard".to_string(),
        ];
        self.function.push_flags(&mut argv);
        self.profile.push_flags(&mut argv);
        self.input.push_flags(&mut argv);
        if let Some(c) = &self.continuation {
            argv.push("--continuation".to_string());
            argv.push(c.clone());
        }
        if let Some(t) = &self.retry_token {
            argv.push("--retry-token".to_string());
            argv.push(t.clone());
        }
        if let Some(seed) = self.seed {
            argv.push("--seed".to_string());
            argv.push(seed.to_string());
        }
        if self.split {
            argv.push("--split".to_string());
        }
        if self.invert {
            argv.push("--invert".to_string());
        }
        if let Some(advanced) = &self.dangerous_advanced {
            argv.push("--dangerous-advanced".to_string());
            argv.push(
                serde_json::to_string(advanced)
                    .expect("RequestDangerousAdvanced serializes"),
            );
        }
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.execute.standard.RequestDangerousAdvanced")]
pub struct RequestDangerousAdvanced {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.functions.execute.standard.ResponseItem")]
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
    /// Retry token from a previous execution.
    #[arg(long)]
    pub retry_token: Option<String>,
    /// Seed for deterministic mock responses.
    #[arg(long)]
    pub seed: Option<i64>,
    /// Treat input as an array and execute once per element.
    #[arg(long)]
    pub split: bool,
    /// Invert outputs after expressions evaluate.
    #[arg(long)]
    pub invert: bool,
    /// Advanced opt-in flags as inline JSON.
    #[arg(long)]
    pub dangerous_advanced: Option<String>,
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
        let dangerous_advanced = if let Some(s) = args.dangerous_advanced {
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
            retry_token: args.retry_token,
            seed: args.seed,
            split: args.split,
            invert: args.invert,
            dangerous_advanced,
            jq: args.jq,
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute_streaming<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<E::Stream<ResponseItem>, E::Error> {
    request.jq = None;
    let mut advanced = request.dangerous_advanced.unwrap_or_default();
    advanced.stream = Some(true);
    request.dangerous_advanced = Some(advanced);
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_streaming_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    jq: String,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<E::Stream<serde_json::Value>, E::Error> {
    request.jq = Some(jq);
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
    request.jq = None;
    if let Some(advanced) = request.dangerous_advanced.as_mut() {
        advanced.stream = None;
    }
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
