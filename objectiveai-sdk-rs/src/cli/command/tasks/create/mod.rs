//! `tasks create` — schedule an ObjectiveAI command. The command runs
//! after `--delay-secs`; with `--repeat` it re-runs every
//! `--delay-secs` (measured from each run's COMPLETION), and
//! `--repeat-count` caps the number of SUCCESSFUL runs (errored runs
//! do not consume the budget — a failing counted repeat retries).
//! The task runs with the identity it was created with — agent
//! arguments and the plugin trio — and its runs carry the
//! daemon-authored `task` identity flag on the `/listen` broadcast.
//!
//! `--command` takes the full command-request JSON (the same shape
//! the `--request` front door accepts); it is stored opaquely and
//! validated daemon-side at create.

use crate::cli::command::CommandRequest;

// No `PartialEq`: the embedded root [`crate::cli::command::Request`]
// doesn't derive it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.create.Request")]
pub struct Request {
    pub path_type: Path,
    /// The command to run — the full typed command request.
    /// Schema-opaque (`serde_json::Value`) ON PURPOSE: embedding the
    /// root request enum's schema in a leaf transitively expands the
    /// whole command tree, which downstream generated TypeScript
    /// cannot emit declarations for (TS7056) — the same reason the
    /// root aggregate is json_schema_ignore'd. The Rust type is the
    /// validation; the wire shape is unchanged.
    #[schemars(with = "serde_json::Value")]
    pub command: Box<crate::cli::command::Request>,
    /// Seconds until the (first) run; with `repeat`, also the interval
    /// between runs, measured from each run's completion.
    pub delay_secs: u64,
    /// Re-run every `delay_secs` instead of running once.
    #[serde(default)]
    pub repeat: bool,
    /// Cap on SUCCESSFUL runs — only valid with `repeat`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub repeat_count: Option<u64>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.create.Path")]
pub enum Path {
    #[serde(rename = "tasks/create")]
    TasksCreate,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// The created task's id.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.create.Response")]
pub struct Response {
    pub id: String,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("command_required").required(true).args(["command"])))]
#[command(group(clap::ArgGroup::new("delay_required").required(true).args(["delay_secs"])))]
pub struct Args {
    /// The command to run: a full command-request JSON object (the
    /// same shape `--request` accepts).
    #[arg(long)]
    pub command: Option<String>,
    /// Seconds until the (first) run; with --repeat, also the interval
    /// between runs.
    #[arg(long)]
    pub delay_secs: Option<u64>,
    /// Re-run every --delay-secs instead of running once.
    #[arg(long)]
    pub repeat: bool,
    /// Cap on successful runs (errored runs don't consume the budget).
    /// Requires --repeat.
    #[arg(long, requires = "repeat")]
    pub repeat_count: Option<u64>,
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
        let command = args.command.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "command",
                "--command is required".to_string(),
            )
        })?;
        // Typed parse — deserialization IS the validation.
        let command: Box<crate::cli::command::Request> =
            serde_json::from_str(&command).map(Box::new).map_err(|e| {
                crate::cli::command::FromArgsError::path_parse(
                    "command",
                    format!("--command is not a valid command request: {e}"),
                )
            })?;
        let delay_secs = args.delay_secs.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse(
                "delay_secs",
                "--delay-secs is required".to_string(),
            )
        })?;
        Ok(Self {
            path_type: Path::TasksCreate,
            command,
            delay_secs,
            repeat: args.repeat,
            repeat_count: args.repeat_count,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
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

/// One `/listen` broadcast run of `tasks create`: the actual
/// [`Request`], the producer's
/// [`AgentArguments`](crate::cli::command::AgentArguments), and the
/// unary response future. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::broadcast_listener::UnaryResponse<Response>,
}
