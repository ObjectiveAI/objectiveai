//! `agents message-queue add` — defer a single user-message-equivalent
//! `RichContent` to a target agent.
//!
//! Stores the content in `tags.sqlite` (the `prompts` table + the
//! per-kind `prompt_<kind>` content tables) against either a
//! resolved `agent_instance_hierarchy` (Direct mode) or a literal
//! `agent_tag` (Tag mode — no resolution at enqueue time; the
//! future reader resolves at dequeue time).
//!
//! The CLI flag surface mirrors `agents message` —
//! `--simple` / `--inline` / `--file` / `--python-inline` /
//! `--python-file` — and reuses the SDK
//! [`super::super::instances::message::RequestMessage`] /
//! [`super::super::instances::message::MessageArgs`] types verbatim so the two
//! leaves' producer plumbing stays in lock-step.
//!
//! This is the write-only slice of #211. No dequeue / flush leaf
//! exists yet — rows persist until a future reader picks them up.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.message_queue.add.Request")]
pub struct Request {
    pub path_type: Path,
    pub target: Target,
    pub message: super::super::instances::message::RequestMessage,
    /// Optional idempotency token (#213). When `Some`, any prior
    /// queued row for the same `(target, key)` pair is overwritten
    /// — old content cascade-dropped, new content inserted with a
    /// fresh `enqueued_at`. Per-target scope: a `key` on a
    /// hierarchy and the same `key` on a tag coexist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub key: Option<String>,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.message_queue.add.Path")]
pub enum Path {
    #[serde(rename = "agents/message-queue/add")]
    AgentsQueueAdd,
}

/// Mutually-exclusive target. `Direct` composes
/// `{parent}/{agent_instance}` at handler time (parent defaults to
/// the cli's own `Config.agent_instance_hierarchy`). `Tag` stores
/// the tag name verbatim — no `tags.sqlite` lookup at enqueue time.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "by", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.message_queue.add.Target")]
pub enum Target {
    #[schemars(title = "Direct")]
    Direct {
        /// Lineage prefix to prepend to `agent_instance`. When
        /// `None`, the CLI substitutes its own
        /// `Config.agent_instance_hierarchy`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        parent_agent_instance_hierarchy: Option<String>,
        /// Leaf id of the target agent.
        agent_instance: String,
    },
    #[schemars(title = "Tag")]
    Tag { agent_tag: String },
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "message-queue".to_string(),
            "add".to_string(),
        ];
        match &self.target {
            Target::Direct {
                parent_agent_instance_hierarchy,
                agent_instance,
            } => {
                argv.push(agent_instance.clone());
                if let Some(parent) = parent_agent_instance_hierarchy {
                    argv.push("--parent-agent-instance-hierarchy".to_string());
                    argv.push(parent.clone());
                }
            }
            Target::Tag { agent_tag } => {
                argv.push("--agent-tag".to_string());
                argv.push(agent_tag.clone());
            }
        }
        self.message.push_flags(&mut argv);
        if let Some(key) = &self.key {
            argv.push("--key".to_string());
            argv.push(key.clone());
        }
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

/// `id` is the row id from `tags.sqlite`'s `prompts` table. Exactly
/// one of `agent_instance_hierarchy` / `agent_tag` is set, matching
/// the chosen [`Target`] variant — `agent_instance_hierarchy` is
/// the **resolved** `{parent}/{instance}` for Direct mode.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.message_queue.add.Response")]
pub struct Response {
    pub id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_instance_hierarchy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_tag: Option<String>,
}

#[derive(clap::Args)]
#[command(group(
    clap::ArgGroup::new("queue_add_target")
        .required(true)
        .multiple(false)
        .args(["agent_instance", "agent_tag"])
))]
#[command(group(
    // `MessageArgs`'s own clap group is now `required = false` —
    // `agents instances message` accepts an empty message. `add`
    // still wants exactly-one, so we layer a per-command group on
    // top to re-impose the required-ness here.
    clap::ArgGroup::new("queue_add_message")
        .required(true)
        .multiple(false)
        .args(["simple", "inline", "file", "python_inline", "python_file"])
))]
pub struct Args {
    /// Leaf id of the target agent. Combined with `--parent` (or
    /// the cli's own `Config.agent_instance_hierarchy` when
    /// omitted) to form the full lineage. Mutually exclusive with
    /// `--agent-tag`.
    pub agent_instance: Option<String>,
    /// Optional lineage prefix to prepend to `agent_instance`.
    /// Only valid alongside `agent_instance`.
    #[arg(long = "parent-agent-instance-hierarchy", requires = "agent_instance")]
    pub parent_agent_instance_hierarchy: Option<String>,
    /// Tag name to enqueue against. Stored verbatim — the cli does
    /// NOT resolve the tag at enqueue time. Mutually exclusive with
    /// `agent_instance` and `--parent-agent-instance-hierarchy`.
    #[arg(long = "agent-tag")]
    pub agent_tag: Option<String>,
    /// Message content input (one of `--simple` / `--inline` /
    /// `--file` / `--python-inline` / `--python-file`). Required.
    #[command(flatten)]
    pub message: super::super::instances::message::MessageArgs,
    /// Optional idempotency token. When set, a second `add` with
    /// the same `(target, key)` overwrites the prior queued row
    /// instead of stacking a new one. Per-target scope — a key on
    /// a hierarchy and the same key on a tag coexist.
    #[arg(long)]
    pub key: Option<String>,
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
        // Same five-arm dispatch `agents message`'s TryFrom uses to
        // normalise the mutually-exclusive MessageArgs fields into
        // one RequestMessage variant.
        let message = if let Some(s) = args.message.simple {
            super::super::instances::message::RequestMessage::Simple(s)
        } else if let Some(s) = args.message.inline {
            let mut de = serde_json::Deserializer::from_str(&s);
            let v = serde_path_to_error::deserialize(&mut de).map_err(|source| {
                crate::cli::command::FromArgsError {
                    field: "inline",
                    source: source.into(),
                }
            })?;
            super::super::instances::message::RequestMessage::Inline(v)
        } else if let Some(p) = args.message.file {
            super::super::instances::message::RequestMessage::File(p)
        } else if let Some(s) = args.message.python_inline {
            super::super::instances::message::RequestMessage::PythonInline(s)
        } else {
            super::super::instances::message::RequestMessage::PythonFile(args.message.python_file.unwrap())
        };
        let target = match (args.agent_instance, args.agent_tag) {
            (Some(agent_instance), None) => Target::Direct {
                parent_agent_instance_hierarchy: args.parent_agent_instance_hierarchy,
                agent_instance,
            },
            (None, Some(agent_tag)) => Target::Tag { agent_tag },
            _ => unreachable!(
                "clap group `queue_add_target` ensures exactly one of agent_instance | agent_tag"
            ),
        };
        Ok(Self {
            path_type: Path::AgentsQueueAdd,
            target,
            message,
            key: args.key,
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
