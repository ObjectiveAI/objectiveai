//! `agents queue deliver` — wake every queue-pending descendant
//! agent of the caller.
//!
//! The handler enumerates two kinds of targets with active queued
//! prompts in the caller's subtree: unique AIHs that are STRICT
//! descendants of the caller (direct rows + rows against BOUND
//! tags), and un-upgraded (GROUPED) tags whose group parent sits in
//! the subtree. Per target it try-acquires the agent's (or tag's)
//! lock with no waiting: a live owner yields
//! [`AgentActiveResponseItem`] / [`TagActiveResponseItem`]; winning
//! the lock yields [`AgentSpawnedResponseItem`] /
//! [`TagSpawnedResponseItem`] and runs the same spawn machinery
//! `agents spawn` / `agents message` use (empty messages, plus the
//! stored continuation for AIHs or the group's stored agent spec for
//! tags), streaming each spawn item as a [`ValueResponseItem`] and
//! releasing the lock when that task's stream ends. Once EVERY
//! target has resolved (active or spawned), the bare string
//! `"AllAgentsActive"` is emitted.
//!
//! Two modes, selected by `dangerous_advanced.stream_spawns`:
//! * unset/false (the default, user-facing): re-exec the cli binary
//!   as a detached orphan with `stream_spawns = true` and emit the
//!   child's items up to and including `AllAgentsActive`, then
//!   return — the orphan keeps running the spawns to completion.
//! * true (the re-exec'd child): run the full delivery in-process and
//!   stream everything, spawn output included.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.deliver.Request")]
pub struct Request {
    pub path_type: Path,
    pub dangerous_advanced: Option<RequestDangerousAdvanced>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.deliver.Path")]
pub enum Path {
    #[serde(rename = "agents/queue/deliver")]
    AgentsQueueDeliver,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.deliver.RequestDangerousAdvanced")]
pub struct RequestDangerousAdvanced {
    /// Run the delivery in-process and stream every spawned agent's
    /// output to completion. Unset/false re-execs a detached child
    /// with this set and returns at its `AllAgentsActive` marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream_spawns: Option<bool>,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "queue".to_string(),
            "deliver".to_string(),
        ];
        if let Some(advanced) = &self.dangerous_advanced {
            argv.push("--dangerous-advanced".to_string());
            argv.push(
                serde_json::to_string(advanced)
                    .expect("RequestDangerousAdvanced serializes"),
            );
        }
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

/// One stream item from `agents queue deliver`. Untagged — the
/// variants are disjoint on the wire: `Value` requires `value`,
/// `AgentActive` / `AgentSpawned` / `TagActive` / `TagSpawned` carry
/// distinct `type` markers, and `AllAgentsActive` is the bare string
/// `"AllAgentsActive"`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.queue.deliver.ResponseItem")]
pub enum ResponseItem {
    #[schemars(title = "Value")]
    Value(ValueResponseItem),
    #[schemars(title = "AgentActive")]
    AgentActive(AgentActiveResponseItem),
    #[schemars(title = "AgentSpawned")]
    AgentSpawned(AgentSpawnedResponseItem),
    #[schemars(title = "TagActive")]
    TagActive(TagActiveResponseItem),
    #[schemars(title = "TagSpawned")]
    TagSpawned(TagSpawnedResponseItem),
    #[schemars(title = "AllAgentsActive")]
    AllAgentsActive(AllAgentsActive),
}

/// One output item from one delivered agent's spawn stream. `value`
/// is the typed root [`crate::cli::command::ResponseItem`] (the spawn
/// item wrapped at the root) — boxed because the root union
/// transitively contains *this* type (`agents → queue → deliver`),
/// and boxing is what makes the recursion sized.
///
/// The `value` field's JSON schema is opaqued to `serde_json::Value`
/// (renders as bare `{}` aka JsonValue) so the published schema
/// doesn't inline the entire root union — that's the TS7056 blowup
/// the root and tier aggregates dodge by being `json_schema_ignore`.
/// Downstream SDKs see `value: JsonValue` on the typed `execute`
/// path; consumers that want to peer inside parse it case-by-case.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.deliver.ValueResponseItem")]
pub struct ValueResponseItem {
    /// The delivered agent's `agent_instance_hierarchy`.
    pub agent_instance_hierarchy: String,
    /// The typed root item the spawn emitted.
    #[schemars(with = "serde_json::Value")]
    pub value: Box<crate::cli::command::ResponseItem>,
}

/// This agent's lock was held by a live owner — it is already active
/// and will drain its own queue; nothing was spawned for it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.deliver.AgentActiveResponseItem")]
pub struct AgentActiveResponseItem {
    pub r#type: AgentActiveType,
    pub agent_instance_hierarchy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.deliver.AgentActiveType")]
pub enum AgentActiveType {
    #[serde(rename = "AgentActive")]
    AgentActive,
}

/// This agent's lock was won and its spawn has started; its output
/// follows as [`ValueResponseItem`]s (in `stream_spawns` mode).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.deliver.AgentSpawnedResponseItem")]
pub struct AgentSpawnedResponseItem {
    pub r#type: AgentSpawnedType,
    pub agent_instance_hierarchy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.deliver.AgentSpawnedType")]
pub enum AgentSpawnedType {
    #[serde(rename = "AgentSpawned")]
    AgentSpawned,
}

/// This un-upgraded tag's lock was held by a live owner — another
/// process is already materializing it; the queued rows will reach
/// the agent it mints. Nothing was spawned for it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.deliver.TagActiveResponseItem")]
pub struct TagActiveResponseItem {
    pub r#type: TagActiveType,
    pub agent_tag: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.deliver.TagActiveType")]
pub enum TagActiveType {
    #[serde(rename = "TagActive")]
    TagActive,
}

/// This un-upgraded tag's lock was won and a fresh spawn of the
/// group's stored agent spec has started. The minted
/// `agent_instance_hierarchy` isn't known yet at this point — it
/// arrives as the FIRST inner item (the spawn `Id`) of the
/// [`ValueResponseItem`]s that follow (in `stream_spawns` mode).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.deliver.TagSpawnedResponseItem")]
pub struct TagSpawnedResponseItem {
    pub r#type: TagSpawnedType,
    pub agent_tag: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.deliver.TagSpawnedType")]
pub enum TagSpawnedType {
    #[serde(rename = "TagSpawned")]
    TagSpawned,
}

/// Every target has resolved to active-or-spawned. Wire shape is the
/// bare string `"AllAgentsActive"` (a one-variant enum — a unit
/// variant in the untagged [`ResponseItem`] would serialize as
/// `null`, not the marker string). The detached default mode stops
/// reading its child at this item.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.queue.deliver.AllAgentsActive")]
pub enum AllAgentsActive {
    AllAgentsActive,
}

#[derive(clap::Args)]
pub struct Args {
    /// Raw JSON for `RequestDangerousAdvanced` (e.g.
    /// `{"stream_spawns":true}`).
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
            path_type: Path::AgentsQueueDeliver,
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
