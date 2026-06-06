//! `agents tasks schedule` — register a command + interval (or
//! oneshot) in `tasks.sqlite`. Add-only leaf; the runner that
//! actually fires schedules is follow-up work tracked by #216.
//!
//! Schedule per row:
//! - `command`: argv vector to invoke on each scheduled poll.
//! - `interval_seconds`: `Some(n)` for a recurring schedule with
//!   `n` seconds as the floor between invocations; `None` for a
//!   **oneshot** that the runner fires once on the next poll and
//!   deletes the row. The CLI gates this via mutually-exclusive
//!   `--interval <humantime>` / `--oneshot` flags.
//! - The caller's full `AgentArguments` snapshot — captured by
//!   the CLI handler so the runner can re-install identity env
//!   vars at fire-time.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tasks.schedule.Request")]
pub struct Request {
    pub path_type: Path,
    /// argv to invoke on each scheduled poll.
    pub command: Vec<String>,
    /// Human-readable label. Required — surfaces on every
    /// `agents tasks list` row, and the runner uses it in
    /// observability output.
    pub description: String,
    /// Floor on wall-clock seconds between invocations. `None`
    /// marks a **oneshot** schedule — the runner fires it once on
    /// the next poll and deletes the row. `Some(n)` is a recurring
    /// schedule with `n` seconds as the minimum gap between
    /// invocations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub interval_seconds: Option<u64>,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tasks.schedule.Path")]
pub enum Path {
    #[serde(rename = "agents/tasks/schedule")]
    AgentsTasksSchedule,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "tasks".to_string(),
            "schedule".to_string(),
        ];
        argv.push("--description".to_string());
        argv.push(self.description.clone());
        match self.interval_seconds {
            Some(secs) => {
                argv.push("--interval".to_string());
                // Round-trip as humantime — `Duration::from_secs(N)`
                // formats as e.g. `30s` / `1h30m` so the CLI
                // re-parses cleanly.
                argv.push(
                    humantime::format_duration(std::time::Duration::from_secs(secs))
                        .to_string(),
                );
            }
            None => argv.push("--oneshot".to_string()),
        }
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        // `--` separator so command argv that itself contains flags
        // round-trips cleanly through the trailing-var-arg parse.
        argv.push("--".to_string());
        argv.extend(self.command.iter().cloned());
        argv
    }
}

/// `id` is the row id from `tasks.sqlite`'s `schedules` table.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tasks.schedule.Response")]
pub struct Response {
    pub id: i64,
}

#[derive(clap::Args)]
#[command(group(
    clap::ArgGroup::new("schedule_kind")
        .required(true)
        .multiple(false)
        .args(["interval", "oneshot"])
))]
pub struct Args {
    /// Minimum interval between scheduled invocations. Humantime
    /// format — `30s`, `5m`, `1h30m`, `2d`. Treated as a floor,
    /// not a wall-clock deadline (#216). Mutually exclusive with
    /// `--oneshot`.
    #[arg(long)]
    pub interval: Option<String>,
    /// Human-readable label for this schedule. Required —
    /// surfaces on every `agents tasks list` row.
    #[arg(long)]
    pub description: String,
    /// Fire the command once on the next harness poll, then
    /// delete the row. Mutually exclusive with `--interval`.
    #[arg(long)]
    pub oneshot: bool,
    /// jq filter applied to the JSON output.
    #[arg(long)]
    pub jq: Option<String>,
    /// Command and arguments to run on each scheduled invocation.
    /// Pass after `--` so flags meant for the inner command don't
    /// collide with the leaf's own (`--interval` / `--oneshot` /
    /// `--jq`).
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 1..)]
    pub command: Vec<String>,
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
        // The `schedule_kind` clap group guarantees exactly one
        // of `--interval` / `--oneshot` is present.
        let interval_seconds = match (args.interval, args.oneshot) {
            (Some(interval), false) => {
                let parsed =
                    humantime::parse_duration(&interval).map_err(|source| {
                        crate::cli::command::FromArgsError {
                            field: "interval",
                            source: source.to_string().into(),
                        }
                    })?;
                Some(parsed.as_secs())
            }
            (None, true) => None,
            _ => unreachable!(
                "clap group `schedule_kind` enforces exactly one of `--interval` | `--oneshot`"
            ),
        };
        if args.command.is_empty() {
            return Err(crate::cli::command::FromArgsError {
                field: "command",
                source: "schedule requires at least one positional argument (the command)"
                    .to_string()
                    .into(),
            });
        }
        Ok(Self {
            path_type: Path::AgentsTasksSchedule,
            command: args.command,
            description: args.description,
            interval_seconds,
            jq: args.jq,
        })
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

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;

pub mod response_schema;
