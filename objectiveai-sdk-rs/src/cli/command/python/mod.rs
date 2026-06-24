//! `python` — run a Python snippet and return its output as JSON.
//!
//! Executes `--code` in the embedded Python runtime with the optional
//! `--input` JSON value exposed to the script as the global `input`,
//! and returns the script's output (its trailing expression value,
//! else captured stdout) as a `serde_json::Value`. No output → JSON
//! `null`.
//!
//! With `--no-objectiveai`, the in-process `objectiveai.execute(...)`
//! host call is disabled: it raises inside the script instead of
//! dispatching a CLI command.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.python.Request")]
pub struct Request {
    pub path_type: Path,
    /// Python source to execute.
    pub code: String,
    /// Optional JSON value exposed to the code as the global `input`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub input: Option<serde_json::Value>,
    /// When set, `objectiveai.execute(...)` inside the embedded python
    /// raises instead of dispatching a CLI command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub no_objectiveai: Option<bool>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.python.Path")]
pub enum Path {
    #[serde(rename = "python")]
    Python,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// Whatever the script produced, as raw JSON (no output → `null`).
pub type Response = serde_json::Value;

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("code_required").required(true).args(["code"])))]
pub struct Args {
    /// Python source to execute.
    #[arg(long)]
    pub code: Option<String>,
    /// Optional JSON value exposed to the code as the global `input`.
    #[arg(long, value_parser = parse_json)]
    pub input: Option<serde_json::Value>,
    /// Disable `objectiveai.execute(...)` inside the script: the host
    /// call raises instead of dispatching a CLI command.
    #[arg(long)]
    pub no_objectiveai: bool,
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

/// `--input` value parser: parse the flag's string as a JSON value.
fn parse_json(s: &str) -> Result<serde_json::Value, String> {
    serde_json::from_str(s).map_err(|e| e.to_string())
}

impl TryFrom<Args> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(args: Args) -> Result<Self, Self::Error> {
        Ok(Self {
            path_type: Path::Python,
            code: args.code.ok_or_else(|| {
                crate::cli::command::FromArgsError::path_parse(
                    "code",
                    "--code is required".to_string(),
                )
            })?,
            input: args.input,
            no_objectiveai: args.no_objectiveai.then_some(true),
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

pub mod request_schema;

pub mod response_schema;
