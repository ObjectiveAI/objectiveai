//! `agents logs read all` — fetch every log row for a target AIH,
//! coalesced into [`ResponseItem`] blocks.
//!
//! Each yielded `ResponseItem` is either a single-row request blob
//! (`AgentCompletionRequest` / `VectorCompletionRequest` /
//! `FunctionExecutionRequest`) or a multi-row block
//! (`ClientNotification` / `AssistantResponse` / `ToolResponse`)
//! formed by joining consecutive `logs.messages` rows that share
//! `(block_class, agent_instance_hierarchy, response_id)` —
//! `ToolResponse` additionally splits per `tool_call_id`.
//!
//! `response_id` is carried on every variant — for the three
//! request-blob variants it's the chunk's own id, for the three
//! block variants it's the immediately-enclosing
//! agent-completion's id (even when the underlying rows were
//! emitted inside a function execution or vector completion
//! chunk).

use std::str::FromStr;

use crate::cli::command::CommandRequest;
use crate::cli::command::path_ref::tokenize;

/// One queue-read target. Either direct `(parent, instance)` (parent
/// defaults to the cli's own `Config.agent_instance_hierarchy` when
/// omitted), a tag name the cli resolves at handler time, or `me`
/// (the caller's own `Config.agent_instance_hierarchy`, with no child
/// leaf appended). Shared with `agents read pending` via re-export.
///
/// Docker-style `key=value,key=value` wire form on the CLI:
///   `--target instance=L`           (direct; parent defaults to ctx)
///   `--target instance=L,parent=P`  (direct; explicit parent)
///   `--target tag=T`                (tag; cli resolves via the tags tier)
///   `--target me`                   (the caller's own AIH; bare valueless key)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "by", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.logs.list.Target")]
pub enum Target {
    #[schemars(title = "Direct")]
    Direct {
        /// Optional lineage prefix. `None` ⇒ cli substitutes
        /// `Config.agent_instance_hierarchy`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        parent_agent_instance_hierarchy: Option<String>,
        /// Leaf id of the target agent.
        agent_instance: String,
    },
    #[schemars(title = "Tag")]
    Tag { agent_tag: String },
    /// The caller's own `Config.agent_instance_hierarchy`, verbatim.
    /// CLI wire form is the bare valueless key `me` (docker
    /// `readonly`-style), mutually exclusive with the other keys.
    #[schemars(title = "Me")]
    Me,
}

impl FromStr for Target {
    type Err = String;
    /// Parse a `--target` arg. Accepted forms: the bare valueless key
    /// `me`; `instance` + optional `parent`; or `tag` alone. `tag` is
    /// mutually exclusive with the other two keys.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Bare valueless key (docker `readonly`-style). Handled before
        // `tokenize`, which requires every token to be `key=value`.
        if s.trim() == "me" {
            return Ok(Target::Me);
        }
        let mut tag: Option<String> = None;
        let mut parent: Option<String> = None;
        let mut instance: Option<String> = None;
        for (k, v) in tokenize(s)? {
            match k {
                "tag" => tag = Some(v.to_string()),
                "instance" => instance = Some(v.to_string()),
                "parent" => parent = Some(v.to_string()),
                other => return Err(format!("unknown key: {other}")),
            }
        }
        match (tag, instance, parent) {
            (Some(t), None, None) => Ok(Target::Tag { agent_tag: t }),
            (Some(_), _, _) => Err(
                "tag is mutually exclusive with instance and parent".to_string(),
            ),
            (None, Some(i), p) => Ok(Target::Direct {
                parent_agent_instance_hierarchy: p,
                agent_instance: i,
            }),
            (None, None, _) => Err("instance or tag is required".to_string()),
        }
    }
}

impl Target {
    /// Inverse of [`FromStr::from_str`]: emit the docker-style
    /// `key=value,key=value` wire form for round-tripping.
    pub fn into_arg_string(&self) -> String {
        match self {
            Target::Me => "me".to_string(),
            Target::Tag { agent_tag } => format!("tag={agent_tag}"),
            Target::Direct {
                parent_agent_instance_hierarchy: None,
                agent_instance,
            } => format!("instance={agent_instance}"),
            Target::Direct {
                parent_agent_instance_hierarchy: Some(p),
                agent_instance,
            } => format!("instance={agent_instance},parent={p}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.list.Request")]
pub struct Request {
    pub path_type: Path,
    /// `true` = list only pending (unfinalized) rows; `false` = list
    /// every logged row. Selected on the CLI by `--pending` / `--all`.
    pub pending: bool,
    pub targets: Vec<Target>,
    /// Skip rows with `logs.messages."index" <= after_id`. Use the
    /// highest `id` from a previous page to paginate forward.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub after_id: Option<i64>,
    /// Cap on rows scanned per target. `None` = unlimited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub limit: Option<i64>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.list.Path")]
pub enum Path {
    #[serde(rename = "agents/logs/list")]
    AgentsLogsList,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

/// Type tag for one `ClientNotification` part — the table-kind of
/// the underlying `message_queue_*` content row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.logs.list.ClientNotificationPartType")]
pub enum ClientNotificationPartType {
    Text,
    Image,
    Audio,
    Video,
    File,
}

/// One row inside a `ClientNotification` block — a consumed
/// `message_queue_contents` entry. `queued_at` is on the
/// enclosing block (it lives on `message_queue.enqueued_at`, not
/// per-content); only the per-row consumption timestamp is here.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.list.ClientNotificationPart")]
pub struct ClientNotificationPart {
    /// `logs.messages."index"` for this row. Pass to
    /// `agents logs read id <n>` to fetch the consumed
    /// `message_queue_contents` body.
    pub id: i64,
    /// `logs.messages."timestamp"` — when the receiver consumed
    /// this content row and the LogWriter committed the
    /// consumption event.
    pub delivered_at: String,
    pub r#type: ClientNotificationPartType,
}

/// One row inside an `AssistantResponse` block, tagged by the
/// table-kind of the underlying `assistant_response_*` row.
///
/// The `ToolCall` variant inlines the call's metadata
/// (`function_name` / `tool_call_id` / `tool_call_index`) so callers
/// can dedupe and correlate without a per-row round-trip; its `id`
/// addresses the same `assistant_response_tool_calls` row, which
/// `agents logs read id <id>` returns as the call's `arguments`
/// (text). Every other variant carries only `id` (the
/// `logs.messages."index"` to pass to `agents logs read id`) and the
/// delivery timestamp.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.logs.list.AssistantResponsePart")]
pub enum AssistantResponsePart {
    #[schemars(title = "ToolCall")]
    ToolCall {
        /// `logs.messages."index"` for the tool-call row. Pass to
        /// `agents logs read id <n>` to read the call's `arguments`
        /// as text.
        id: i64,
        delivered_at: String,
        /// `objectiveai.assistant_response_tool_calls.function_name`.
        function_name: String,
        /// The wire tool-call id this row carries.
        tool_call_id: String,
        /// The tool call's wire index within the assistant message's
        /// `tool_calls[]`.
        tool_call_index: i64,
    },
    #[schemars(title = "Refusal")]
    Refusal { id: i64, delivered_at: String },
    #[schemars(title = "Reasoning")]
    Reasoning { id: i64, delivered_at: String },
    #[schemars(title = "Text")]
    Text { id: i64, delivered_at: String },
    #[schemars(title = "Image")]
    Image { id: i64, delivered_at: String },
    #[schemars(title = "Audio")]
    Audio { id: i64, delivered_at: String },
    #[schemars(title = "Video")]
    Video { id: i64, delivered_at: String },
    #[schemars(title = "File")]
    File { id: i64, delivered_at: String },
}

/// Type tag for one `ToolResponse` part — the table-kind of the
/// underlying `tool_response_content_*` row. The tool-call linkage
/// (`tool_call_id`) lives on the enclosing `ResponseItem::ToolResponse`
/// block, not on the parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.logs.list.ToolResponsePartType")]
pub enum ToolResponsePartType {
    Text,
    Image,
    Audio,
    Video,
    File,
}

/// One row inside a `ToolResponse` block.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.list.ToolResponsePart")]
pub struct ToolResponsePart {
    /// `logs.messages."index"` for this row. Pass to
    /// `agents logs read id <n>` for the typed body.
    pub id: i64,
    pub delivered_at: String,
    pub r#type: ToolResponsePartType,
}

/// Type tag for one `RequestMessageUser` part — the kind of the
/// underlying `request_message_user_content_*` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.logs.list.RequestMessageUserPartType")]
pub enum RequestMessageUserPartType {
    Text,
    Image,
    Audio,
    Video,
    File,
}

/// One content part of a user request message. Its `id` addresses the
/// stored content the same way a response part's `id` does — pass it
/// to `agents logs read id <n>`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.list.RequestMessageUserPart")]
pub struct RequestMessageUserPart {
    /// `logs.messages."index"` for this row.
    pub id: i64,
    pub delivered_at: String,
    pub r#type: RequestMessageUserPartType,
}

/// Type tag for one `VectorRequestChoices` choice part — the kind of
/// the underlying `request_vector_choice_content_*` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.logs.list.VectorRequestChoicePartType")]
pub enum VectorRequestChoicePartType {
    Text,
    Image,
    Audio,
    Video,
    File,
}

/// One content part of a vector-completion request choice. `id`
/// addresses the stored content via `agents logs read id <n>`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.list.VectorRequestChoicePart")]
pub struct VectorRequestChoicePart {
    /// `logs.messages."index"` for this row.
    pub id: i64,
    pub delivered_at: String,
    pub r#type: VectorRequestChoicePartType,
}

/// One choice in a `VectorRequestChoices` block: this agent's inline
/// voting `key` for the choice plus the choice's content parts (in
/// request order). The key is returned inline — no `read id` needed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.list.VectorRequestChoice")]
pub struct VectorRequestChoice {
    /// The prefix-tree voting key this agent assigned to the choice.
    pub key: String,
    pub parts: Vec<VectorRequestChoicePart>,
}

/// One yielded item. Three single-row request blobs +
/// three multi-row blocks. Every variant carries `response_id`.
/// `sender_agent_instance_hierarchy` appears only on the four
/// variants that have a sender ≠ producer: the three request
/// variants (caller AIH) and `ClientNotification` (enqueuer
/// AIH). `AssistantResponse` and `ToolResponse` are emitted BY
/// the agent itself — their `agent_instance_hierarchy` IS the
/// producer, so no separate sender field exists.
///
/// Block-coalescing boundary tuple: `(class,
/// agent_instance_hierarchy, response_id)` for `AssistantResponse`;
/// `(class, agent_instance_hierarchy, response_id, tool_call_id)`
/// for `ToolResponse` (one block per tool call); `(class,
/// agent_instance_hierarchy, response_id, sender, message_queue_id)`
/// for `ClientNotification` blocks.
/// One `ClientNotification` block = one consumed
/// `message_queue` parent row, so `queued_at` and
/// `sender_agent_instance_hierarchy` are well-defined
/// block-level.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.logs.list.ResponseItem")]
pub enum ResponseItem {
    /// A `user`-role message from the request/task input, unpacked
    /// into content parts (each addressable via `read id`).
    #[schemars(title = "RequestMessageUser")]
    RequestMessageUser {
        agent_instance_hierarchy: String,
        response_id: String,
        parts: Vec<RequestMessageUserPart>,
    },
    /// An `assistant`-role message from the request/task input. Same
    /// part shape as an assistant response — reasoning / tool calls /
    /// refusal / content — but sourced from the request.
    #[schemars(title = "RequestMessageAssistant")]
    RequestMessageAssistant {
        agent_instance_hierarchy: String,
        response_id: String,
        parts: Vec<AssistantResponsePart>,
    },
    /// A `tool`-role message from the request/task input, answering a
    /// prior tool call. Same part shape as a tool response.
    #[schemars(title = "RequestMessageTool")]
    RequestMessageTool {
        agent_instance_hierarchy: String,
        response_id: String,
        /// The wire tool-call id this request message answers.
        tool_call_id: String,
        parts: Vec<ToolResponsePart>,
    },
    /// The response choices a vector-completion task voted over,
    /// yielded as one block: an ordered list of choices, each with its
    /// content parts and this agent's inline voting `key`.
    #[schemars(title = "VectorRequestChoices")]
    VectorRequestChoices {
        agent_instance_hierarchy: String,
        response_id: String,
        choices: Vec<VectorRequestChoice>,
    },
    /// The closer for a function-execution vector task: this agent's
    /// own vote (its score for each choice, in choice order). Inline —
    /// no `read id` needed.
    #[schemars(title = "VectorResponseVote")]
    VectorResponseVote {
        agent_instance_hierarchy: String,
        response_id: String,
        #[schemars(with = "Vec<f64>")]
        vote: Vec<rust_decimal::Decimal>,
    },
    #[schemars(title = "ClientNotification")]
    ClientNotification {
        agent_instance_hierarchy: String,
        /// AIH of the enqueuer — from `message_queue.sender_*`
        /// joined through `message_queue_contents.id`.
        sender_agent_instance_hierarchy: String,
        response_id: String,
        /// `message_queue.enqueued_at` of the consumed parent
        /// queue row. One block = one parent queue row, so this
        /// is well-defined block-level (each part's individual
        /// `delivered_at` still records its own
        /// consumption moment).
        queued_at: String,
        /// Idempotency token, if the row was enqueued with
        /// `--key` via `agents message --enqueue-with-key`.
        /// Surfacing it lets readers attribute a notification
        /// to a specific enqueue beyond just the sender AIH.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        key: Option<String>,
        parts: Vec<ClientNotificationPart>,
    },
    /// Agent emissions — the agent IS the producer of these
    /// rows, so there's no separate sender. The
    /// `agent_instance_hierarchy` field IS the sender.
    #[schemars(title = "AssistantResponse")]
    AssistantResponse {
        agent_instance_hierarchy: String,
        response_id: String,
        parts: Vec<AssistantResponsePart>,
    },
    /// One tool call's response. Blocks are grouped per
    /// `tool_call_id` (in addition to `agent_instance_hierarchy` +
    /// `response_id`), so two responses in the same turn yield two
    /// blocks.
    #[schemars(title = "ToolResponse")]
    ToolResponse {
        agent_instance_hierarchy: String,
        response_id: String,
        /// The wire tool-call id this response answers.
        tool_call_id: String,
        parts: Vec<ToolResponsePart>,
    },
    /// One logged failure (`objectiveai.errors`) — a spawn-path error
    /// persisted into the agent's history. Always a single row: the
    /// failing attempt dies at its first raised error, so there is
    /// nothing to coalesce. The value is returned inline (like the
    /// vote) — no `read id` needed — but the row is still
    /// id-addressable like every other event.
    #[schemars(title = "Error")]
    Error {
        agent_instance_hierarchy: String,
        /// The response the failure belongs to when one existed;
        /// `None` for post-lock pre-stream failures (the only kind
        /// allowed a NULL response_id).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        response_id: Option<String>,
        /// `logs.messages."index"` for this row.
        id: i64,
        delivered_at: String,
        /// The CLI's user-facing error value — a structured object for
        /// API response errors, a plain string otherwise.
        error: serde_json::Value,
    },
}

#[derive(clap::Args)]
#[command(group(
    clap::ArgGroup::new("logs_list_mode")
        .required(true)
        .multiple(false)
        .args(["all", "pending"])
))]
pub struct Args {
    /// List every logged row. Mutually exclusive with `--pending`;
    /// exactly one of the two is required.
    #[arg(long)]
    pub all: bool,
    /// List only pending (unfinalized) rows. Mutually exclusive with
    /// `--all`; exactly one of the two is required.
    #[arg(long)]
    pub pending: bool,
    /// One or more `--target instance=L[,parent=P]` entries. `parent`
    /// defaults to the cli's own `Config.agent_instance_hierarchy`
    /// when omitted on an individual target. Also accepts
    /// `--target tag=T` and `--target me` (the caller's own AIH).
    #[arg(long = "target", required = true)]
    pub targets: Vec<String>,
    /// Skip rows with `logs.messages."index" <= after_id` per target.
    #[arg(long)]
    pub after_id: Option<i64>,
    /// Cap on rows scanned per target.
    #[arg(long)]
    pub limit: Option<i64>,
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
        let targets = args
            .targets
            .iter()
            .map(|s| {
                s.parse::<Target>().map_err(|msg| {
                    crate::cli::command::FromArgsError::path_parse("target", msg)
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            path_type: Path::AgentsLogsList,
            // The `logs_list_mode` group guarantees exactly one of
            // `--all` / `--pending`, so `pending` is the full mode.
            pending: args.pending,
            targets,
            after_id: args.after_id,
            limit: args.limit,
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

#[cfg(test)]
mod tests {
    use super::Target;

    #[test]
    fn me_target_parses_and_round_trips() {
        assert_eq!("me".parse::<Target>(), Ok(Target::Me));
        // Surrounding whitespace is trimmed, same as the other forms.
        assert_eq!("  me  ".parse::<Target>(), Ok(Target::Me));
        assert_eq!(Target::Me.into_arg_string(), "me");
    }

    #[test]
    fn me_is_mutually_exclusive_with_other_keys() {
        // `me` combined with any other key falls through to the
        // `key=value` tokenizer and is rejected.
        assert!("me,instance=x".parse::<Target>().is_err());
    }

    #[test]
    fn json_wire_shape_is_by_me() {
        assert_eq!(
            serde_json::to_value(Target::Me).unwrap(),
            serde_json::json!({ "by": "me" }),
        );
    }
}

/// One `/listen` broadcast run of `agents logs list`: the actual
/// [`Request`], the producer's
/// [`AgentArguments`](crate::cli::command::AgentArguments), and the
/// response-item stream. See [`crate::cli::websocket_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::websocket_listener::ResponseItemStream<ResponseItem>,
}
