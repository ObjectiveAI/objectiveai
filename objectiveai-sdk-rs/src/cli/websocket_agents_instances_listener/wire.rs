//! Wire types for the daemon `/agents/instances/{*aih}` endpoint.
//!
//! **The conversation types are a MIRROR of `agents logs list`'s
//! [`ResponseItem`](crate::cli::command::agents::logs::list::ResponseItem)
//! family** — same blocks, same fields — with one mechanical
//! replacement: wherever a list part carries an `id` addressing its
//! content via `agents logs open`, the mirror carries the OPENED
//! content itself (and the `type` tag folds into the typed content).
//! A consumer who knows the logs-list shapes already knows these.
//!
//! Transport: the stream ships one typed event per conversation row —
//! each event names its block class, carries the block's boundary
//! fields (exactly the fields `ResponseItem` keeps at block level),
//! ONE mirrored part, and the row's DB identity `(row_index,
//! row_sub_index)` as an opaque replace-at key. Events are keyed
//! FULL-VALUE upserts — never deltas: a re-sent identity REPLACES the
//! prior part (per-connection ordering means later = more complete),
//! which is also how the DB-snapshot replay and the live tee converge.

use crate::agent::completions::message::{File, ImageUrl, InputAudio, VideoUrl};

/// One agent's record: identity, spawn / last-active timestamps, tag
/// bindings, counters, and whether its per-instance lock is currently
/// held. Mirrors `agents instances list`'s `ResponseItem` plus the
/// live `active` flag. Carried by [`AgentInstanceEvent::Agent`] — the
/// `/agents/instances/list` stream itself is a flat list of AIH
/// strings and carries no records.
#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.websocket_agents_instances_listener.AgentRecord")]
pub struct AgentRecord {
    /// Full hierarchy of this agent instance.
    pub agent_instance_hierarchy: String,
    /// Tag names currently bound to this AIH, newest-bound first.
    pub tags: Vec<String>,
    /// Active `message_queue` rows targeting this agent.
    pub queued: u64,
    /// Total `objectiveai.messages` rows for this agent over all time.
    pub logged: u64,
    /// Whether the agent's per-instance lock is currently held — i.e. a
    /// live process owns this agent right now.
    pub active: bool,
    /// RFC3339 timestamp of the first `objectiveai.messages` row for this
    /// agent (spawn time). `None` for an agent with no logs yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub spawned_at: Option<String>,
    /// RFC3339 timestamp the agent was last active. Meaningful only when
    /// `active` is `false` — a live agent's last-active is implicitly
    /// "now", so it is left `None` while active and stamped at the moment
    /// the lock releases.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub last_active_at: Option<String>,
}

/// One part's opened content — what `agents logs read id` would return
/// for the part's `id`, inlined. The mirror of the list part `type`
/// tags (`text` / `image` / `audio` / `video` / `file`), each carrying
/// its payload.
#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.websocket_agents_instances_listener.PartContent")]
pub enum PartContent {
    #[schemars(title = "Text")]
    Text { text: String },
    #[schemars(title = "Image")]
    Image(ImageUrl),
    #[schemars(title = "Audio")]
    Audio(InputAudio),
    #[schemars(title = "Video")]
    Video(VideoUrl),
    #[schemars(title = "File")]
    File(File),
}

/// Mirror of `logs.list.ClientNotificationPart`: `id` + `type` →
/// the opened content. `queued_at` stays on the enclosing block.
#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.websocket_agents_instances_listener.ClientNotificationPart")]
pub struct ClientNotificationPart {
    /// When the receiver consumed this content row.
    pub delivered_at: String,
    pub content: PartContent,
}

/// Mirror of `logs.list.AssistantResponsePart`: each variant keeps its
/// list fields with `id` replaced by the opened content — `arguments`
/// for a tool call, `text` for refusal / reasoning, the typed payload
/// for content parts.
#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.websocket_agents_instances_listener.AssistantResponsePart")]
pub enum AssistantResponsePart {
    /// A tool call. `arguments` is the call's full current accumulated
    /// string (full-value, not a fragment).
    #[schemars(title = "ToolCall")]
    ToolCall {
        delivered_at: String,
        function_name: String,
        /// The wire tool-call id this row carries.
        tool_call_id: String,
        /// The tool call's wire index within the assistant message's
        /// `tool_calls[]`.
        tool_call_index: i64,
        arguments: String,
    },
    #[schemars(title = "Refusal")]
    Refusal { delivered_at: String, text: String },
    #[schemars(title = "Reasoning")]
    Reasoning { delivered_at: String, text: String },
    #[schemars(title = "Text")]
    Text { delivered_at: String, text: String },
    #[schemars(title = "Image")]
    Image {
        delivered_at: String,
        image: ImageUrl,
    },
    #[schemars(title = "Audio")]
    Audio {
        delivered_at: String,
        audio: InputAudio,
    },
    #[schemars(title = "Video")]
    Video {
        delivered_at: String,
        video: VideoUrl,
    },
    #[schemars(title = "File")]
    File { delivered_at: String, file: File },
}

/// Mirror of `logs.list.ToolResponsePart`: `id` + `type` → the opened
/// content. The tool-call linkage (`tool_call_id`) lives on the
/// enclosing block, exactly as in the list shape.
#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.websocket_agents_instances_listener.ToolResponsePart")]
pub struct ToolResponsePart {
    pub delivered_at: String,
    pub content: PartContent,
}

/// Mirror of `logs.list.RequestMessageUserPart`: `id` + `type` → the
/// opened content.
#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.websocket_agents_instances_listener.RequestMessageUserPart")]
pub struct RequestMessageUserPart {
    pub delivered_at: String,
    pub content: PartContent,
}

/// Mirror of `logs.list.VectorRequestChoicePart`: `id` + `type` → the
/// opened content.
#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.websocket_agents_instances_listener.VectorRequestChoicePart")]
pub struct VectorRequestChoicePart {
    pub delivered_at: String,
    pub content: PartContent,
}

/// Mirror of `logs.list.VectorRequestChoice`: this agent's inline
/// voting `key` for the choice plus the choice's content parts in
/// request order.
#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.websocket_agents_instances_listener.VectorRequestChoice")]
pub struct VectorRequestChoice {
    /// The prefix-tree voting key this agent assigned to the choice.
    pub key: String,
    pub parts: Vec<VectorRequestChoicePart>,
}

/// One materialized conversation block — **the mirror of
/// `agents logs list`'s `ResponseItem`**, variant-for-variant and
/// field-for-field, with parts carrying opened content instead of
/// `read id` addresses; plus the additional `Error` block (a logged
/// failure — the list gains the same variant). Produced by the
/// listener's incremental coalescer; blocks appear in conversation
/// order with `read_all`'s exact boundary rule.
#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.websocket_agents_instances_listener.ConversationBlock")]
pub enum ConversationBlock {
    /// A `user`-role message from the request/task input.
    #[schemars(title = "RequestMessageUser")]
    RequestMessageUser {
        agent_instance_hierarchy: String,
        response_id: String,
        parts: Vec<RequestMessageUserPart>,
    },
    /// An `assistant`-role message from the request/task input.
    #[schemars(title = "RequestMessageAssistant")]
    RequestMessageAssistant {
        agent_instance_hierarchy: String,
        response_id: String,
        parts: Vec<AssistantResponsePart>,
    },
    /// A `tool`-role message from the request/task input, answering a
    /// prior tool call.
    #[schemars(title = "RequestMessageTool")]
    RequestMessageTool {
        agent_instance_hierarchy: String,
        response_id: String,
        /// The wire tool-call id this request message answers.
        tool_call_id: String,
        parts: Vec<ToolResponsePart>,
    },
    /// The response choices a vector-completion task voted over.
    #[schemars(title = "VectorRequestChoices")]
    VectorRequestChoices {
        agent_instance_hierarchy: String,
        response_id: String,
        choices: Vec<VectorRequestChoice>,
    },
    /// The closer for a vector task: this agent's own vote (its score
    /// for each choice, in choice order).
    #[schemars(title = "VectorResponseVote")]
    VectorResponseVote {
        agent_instance_hierarchy: String,
        response_id: String,
        #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
        #[schemars(with = "Vec<f64>")]
        vote: Vec<rust_decimal::Decimal>,
    },
    /// Consumed message-queue notifications from one parent queue row.
    #[schemars(title = "ClientNotification")]
    ClientNotification {
        agent_instance_hierarchy: String,
        /// AIH of the enqueuer.
        sender_agent_instance_hierarchy: String,
        response_id: String,
        /// `message_queue.enqueued_at` of the consumed parent queue
        /// row — one block = one parent row.
        queued_at: String,
        /// Idempotency token, if the row was enqueued with `--key`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        key: Option<String>,
        parts: Vec<ClientNotificationPart>,
    },
    /// The agent's own assistant output.
    #[schemars(title = "AssistantResponse")]
    AssistantResponse {
        agent_instance_hierarchy: String,
        response_id: String,
        parts: Vec<AssistantResponsePart>,
    },
    /// One tool call's response — one block per `tool_call_id`.
    #[schemars(title = "ToolResponse")]
    ToolResponse {
        agent_instance_hierarchy: String,
        response_id: String,
        /// The wire tool-call id this response answers.
        tool_call_id: String,
        parts: Vec<ToolResponsePart>,
    },
    /// One logged failure, value inline — an error is a single row,
    /// never a parts-carrying container (the failing attempt dies at
    /// its first raised error), so each error row is its own block.
    #[schemars(title = "Error")]
    Error {
        agent_instance_hierarchy: String,
        /// The response the failure belongs to when one existed;
        /// `None` for post-lock pre-stream failures.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        response_id: Option<String>,
        /// The CLI's user-facing error value — a structured object for
        /// API response errors, a plain string otherwise.
        error: serde_json::Value,
        /// RFC3339 — when the failure was logged.
        delivered_at: String,
    },
}

/// One event on the `/agents/instances/{*aih}` stream. The
/// conversation events carry ONE mirrored part each, addressed by
/// their block's boundary fields (exactly the fields `ResponseItem`
/// keeps at block level) plus the row's DB identity — `row_index` /
/// `row_sub_index` are `objectiveai.messages`' values, an OPAQUE
/// replace-at + ordering key: a re-sent identity replaces the prior
/// part, both within the snapshot/live seam and across streaming
/// updates of one part. Single-row blocks (`vote` / `error`) ship
/// complete.
///
/// The DB snapshot replays first (in `objectiveai.messages."index"`
/// order), one `Live` marks the snapshot complete, then live events
/// follow as the conversation occurs. `Agent` events (the status
/// record) are structurally independent and can arrive at any time.
///
/// Extensibility contract: future variants WILL be added — consumers
/// must skip events they cannot parse.
#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.websocket_agents_instances_listener.AgentInstanceEvent")]
pub enum AgentInstanceEvent {
    /// One part of a `RequestMessageUser` block. `row_index` = message
    /// index, `row_sub_index` = part index.
    #[schemars(title = "RequestMessageUserPart")]
    RequestMessageUserPart {
        agent_instance_hierarchy: String,
        response_id: String,
        row_index: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        row_sub_index: Option<i64>,
        part: RequestMessageUserPart,
    },
    /// One part of a `RequestMessageAssistant` block.
    #[schemars(title = "RequestMessageAssistantPart")]
    RequestMessageAssistantPart {
        agent_instance_hierarchy: String,
        response_id: String,
        row_index: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        row_sub_index: Option<i64>,
        part: AssistantResponsePart,
    },
    /// One part of a `RequestMessageTool` block.
    #[schemars(title = "RequestMessageToolPart")]
    RequestMessageToolPart {
        agent_instance_hierarchy: String,
        response_id: String,
        /// The wire tool-call id the block answers (block boundary).
        tool_call_id: String,
        row_index: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        row_sub_index: Option<i64>,
        part: ToolResponsePart,
    },
    /// One part of one choice of a `VectorRequestChoices` block.
    #[schemars(title = "VectorRequestChoicePart")]
    VectorRequestChoicePart {
        agent_instance_hierarchy: String,
        response_id: String,
        /// This agent's inline voting key for the choice.
        key: String,
        /// The choice's index within the request (`row_index`).
        choice_index: i64,
        /// The part's index within the choice (`row_sub_index`).
        part_index: i64,
        part: VectorRequestChoicePart,
    },
    /// A complete `VectorResponseVote` block (single-row).
    #[schemars(title = "VectorResponseVote")]
    VectorResponseVote {
        agent_instance_hierarchy: String,
        response_id: String,
        #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
        #[schemars(with = "Vec<f64>")]
        vote: Vec<rust_decimal::Decimal>,
    },
    /// One part of a `ClientNotification` block. `row_index` = the
    /// consumed `message_queue_contents.id`.
    #[schemars(title = "ClientNotificationPart")]
    ClientNotificationPart {
        agent_instance_hierarchy: String,
        response_id: String,
        /// AIH of the enqueuer (block boundary, with
        /// `message_queue_id`).
        sender_agent_instance_hierarchy: String,
        /// `message_queue.id` of the consumed parent queue row —
        /// one block per parent row.
        message_queue_id: i64,
        /// `message_queue.enqueued_at` of the parent row.
        queued_at: String,
        /// Idempotency token, if the row was enqueued with `--key`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        key: Option<String>,
        row_index: i64,
        part: ClientNotificationPart,
    },
    /// One part of an `AssistantResponse` block.
    #[schemars(title = "AssistantResponsePart")]
    AssistantResponsePart {
        agent_instance_hierarchy: String,
        response_id: String,
        row_index: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        row_sub_index: Option<i64>,
        part: AssistantResponsePart,
    },
    /// One part of a `ToolResponse` block.
    #[schemars(title = "ToolResponsePart")]
    ToolResponsePart {
        agent_instance_hierarchy: String,
        response_id: String,
        /// The wire tool-call id the block answers (block boundary).
        tool_call_id: String,
        row_index: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        row_sub_index: Option<i64>,
        part: ToolResponsePart,
    },
    /// A complete `Error` block. Errors are IMMUTABLE and single-shot
    /// — never updated, so they carry no replace-at identity;
    /// consumers dedupe the snapshot/live seam by value equality.
    #[schemars(title = "Error")]
    Error {
        agent_instance_hierarchy: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        response_id: Option<String>,
        error: serde_json::Value,
        delivered_at: String,
    },
    /// The snapshot is complete; every following event is live.
    #[schemars(title = "Live")]
    Live,
    /// This agent's list record — the same shape
    /// `/agents/instances/list` tracks (lock-driven `active` flag,
    /// bound tags, counters). A FULL-VALUE upsert: sent once on
    /// connect with the current state, then re-sent whenever it
    /// changes. Structurally independent of the conversation events.
    #[schemars(title = "Agent")]
    Agent { agent: AgentRecord },
}
