//! `agents tasks run` — fire every pending schedule in the caller's
//! own subtree.
//!
//! Scope is fixed: every schedule whose `agent_instance_hierarchy` is
//! the caller's own AIH or a descendant of it. Of those, the runner
//! claims the pending ones (unfired oneshots + interval rows whose
//! interval has elapsed — newest versions only), minting one
//! `tasks_runs` row per firing, and dispatches each row's stored argv
//! through the root `crate::run` in parallel — with the schedule's
//! captured identity (and the plugin that registered it, if any)
//! re-installed on the run ctx.
//!
//! Two output modes, selected by `dangerous_advanced.stream_all`:
//! * `--dangerous-advanced '{"stream_all":true}'`: every item every
//!   task emits streams back, each wrapped as a [`ValueResponseItem`].
//! * default: exactly ONE [`SuccessResponseItem`] per task, emitted
//!   when that task's stream completes — `success` is `false` iff the
//!   task's final item was an error.
//!
//! `stream_all` is gated behind `dangerous_advanced` (rather than a
//! bare top-level flag) because streaming every item of every fired
//! task can bloat the caller's context astronomically.
//!
//! The mode affects only the caller-visible stream: in BOTH modes the
//! full per-item output is durably written to `tasks_logs`.

use crate::cli::command::CommandRequest;

/// The plugin that registered a schedule — the same shape `tasks list`
/// surfaces.
pub use crate::cli::command::tasks::list::Plugin;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.run.Request")]
pub struct Request {
    pub path_type: Path,
    pub dangerous_advanced: Option<RequestDangerousAdvanced>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.run.Path")]
pub enum Path {
    #[serde(rename = "tasks/run")]
    AgentsTasksRun,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// Advanced knobs gated behind `--dangerous-advanced` because they are
/// easy to misuse. Modelled on `agents spawn`'s
/// `RequestDangerousAdvanced`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.run.RequestDangerousAdvanced")]
pub struct RequestDangerousAdvanced {
    /// Stream every item every fired task emits (each a
    /// [`ValueResponseItem`]). When unset/false — the default — each
    /// task yields exactly one [`SuccessResponseItem`] summary instead;
    /// the full output still lands in `tasks_logs` either way. DANGER:
    /// streaming every item of every fired task can bloat the caller's
    /// context astronomically — hence the gate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream_all: Option<bool>,
}

/// One stream item from `tasks run`. Untagged — the variants'
/// required fields (`value` vs `success`) are disjoint, so the wire
/// shape is just the inner object. Which variant flows is decided by
/// the request's `stream_all`: `true` streams every emitted item as a
/// [`ValueResponseItem`] (whose `value` is the typed root item for a
/// no-transform command, or the post-transform JSON otherwise — see
/// [`RunValue`]); `false` (default) yields exactly one
/// [`SuccessResponseItem`] per task when its stream completes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.tasks.run.ResponseItem")]
pub enum ResponseItem {
    #[schemars(title = "Value")]
    Value(ValueResponseItem),
    #[schemars(title = "Success")]
    Success(SuccessResponseItem),
}

/// One output item from one fired schedule's in-process stream
/// (`stream_all` mode). The first four fields identify the source
/// schedule; `value` is the typed root
/// [`crate::cli::command::ResponseItem`] emitted by the scheduled cli
/// leaf — boxed because the root union transitively contains *this*
/// type (`agents → tasks → run`), and boxing is what makes the
/// recursion sized.
///
/// The `value` field's JSON schema is opaqued to `serde_json::Value`
/// (renders as bare `{}` aka JsonValue) so the published schema
/// doesn't inline the entire root union — that's the TS7056 blowup
/// the root and tier aggregates dodge by being `json_schema_ignore`.
/// Downstream SDKs see `value: JsonValue` on the typed `execute`
/// path; consumers that want to peer inside parse it case-by-case.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.run.ValueResponseItem")]
pub struct ValueResponseItem {
    /// The source schedule's `agent_instance_hierarchy`.
    pub agent_instance_hierarchy: String,
    /// The source schedule's `--name`.
    pub name: String,
    /// The source schedule's version (`1` on first creation,
    /// incremented per `schedule --overwrite`).
    pub version: u64,
    /// The plugin that registered the source schedule, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub plugin: Option<Plugin>,
    /// What the scheduled command emitted — either the typed root item
    /// (no transform) or post-transform JSON. See [`RunValue`]. Schema
    /// is opaqued to `serde_json::Value`.
    #[schemars(with = "serde_json::Value")]
    pub value: RunValue,
}

/// The per-item value a fired schedule emits, mirroring the two root
/// dispatch paths at the item level (untagged — the wire shape is just
/// the inner value):
/// - [`RunValue::ExecuteValue`]: the typed root
///   [`crate::cli::command::ResponseItem`] from a no-transform command.
///   Boxed because the root union transitively contains *this* type
///   (`agents → tasks → run`), and its schema is opaqued to
///   `serde_json::Value` so the published schema doesn't inline the
///   entire root union (the TS7056 blowup the aggregates dodge).
/// - [`RunValue::ExecuteTransformValue`]: the post-transform JSON from
///   a command that carried a `--jq` / `--python` transform.
///
/// Deliberately does NOT derive `JsonSchema` and is `json_schema_ignore`d:
/// its only use site ([`ValueResponseItem::value`]) opaques it to
/// `serde_json::Value`, so its own schema is never referenced. Deriving
/// it would publish a degenerate `anyOf` of two type-less `{}` variants
/// (both arms are wire-opaque), which no SDK code generator can name.
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum RunValue {
    ExecuteValue(Box<crate::cli::command::ResponseItem>),
    ExecuteTransformValue(serde_json::Value),
}

/// One per-task completion summary (default mode): the same schedule
/// identity as [`ValueResponseItem`], with `success` in lieu of
/// `value`. `success` is `false` iff the task's FINAL emitted item was
/// an error (a task that emitted nothing is a success). The task's
/// full output is in `tasks_logs` regardless.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.run.SuccessResponseItem")]
pub struct SuccessResponseItem {
    /// The source schedule's `agent_instance_hierarchy`.
    pub agent_instance_hierarchy: String,
    /// The source schedule's `--name`.
    pub name: String,
    /// The source schedule's version (`1` on first creation,
    /// incremented per `schedule --overwrite`).
    pub version: u64,
    /// The plugin that registered the source schedule, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub plugin: Option<Plugin>,
    /// Whether the task's stream completed without a trailing error.
    pub success: bool,
}

#[derive(clap::Args)]
pub struct Args {
    /// Raw JSON for `RequestDangerousAdvanced` (e.g.
    /// `{"stream_all":true}`). `stream_all` is gated here — rather than
    /// as a bare flag — because streaming every item of every fired
    /// task can bloat the caller's context astronomically.
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
        let dangerous_advanced: Option<RequestDangerousAdvanced> =
            if let Some(s) = args.dangerous_advanced {
                let mut de = serde_json::Deserializer::from_str(&s);
                let v = serde_path_to_error::deserialize(&mut de).map_err(|source| {
                    crate::cli::command::FromArgsError {
                        field: "dangerous_advanced",
                        source: source.into(),
                    }
                })?;
                Some(v)
            } else {
                None
            };
        Ok(Self {
            path_type: Path::AgentsTasksRun,
            dangerous_advanced,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<E::Stream<ResponseItem>, E::Error> {
    request.base.clear_transform();
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    transform: crate::cli::command::Transform,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<E::Stream<serde_json::Value>, E::Error> {
    request.base.set_transform(transform);
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;

pub mod response_schema;
