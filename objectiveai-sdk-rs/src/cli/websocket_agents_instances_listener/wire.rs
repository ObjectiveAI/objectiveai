//! Wire types for the daemon `/agents/instances/{*aih}` endpoint.
//!
//! The stream is keyed FULL-VALUE upserts — never deltas. Each
//! [`ConversationRow`] carries its complete current content; a row
//! re-sent under the same identity `(table, response_id, row_index,
//! row_sub_index)` REPLACES the prior one (per-connection ordering
//! means later = more complete). The `objectiveai.messages."index"`
//! content id is deliberately absent: live rows are shipped before
//! (and independent of) their DB insert, so the id is unknowable —
//! identity is the writer-assigned key above, byte-identical between
//! the DB snapshot and the live tee.

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

/// Which logs table a [`ConversationRow`] mirrors — the client's
/// block-class discriminant. The 42 `objectiveai.message_table` event
/// kinds (request blobs excluded — they never appear on this stream),
/// plus the three HEAD rows (`tool_response`, `request_message_tool`,
/// `request_vector_choice`) that emit no `objectiveai.messages` event but
/// carry block metadata: the two tool heads carry the `tool_call_id`
/// their block answers, and `request_vector_choice` carries this
/// agent's inline voting `key` for one choice. (No per-variant docs —
/// they would break the plain string-enum schema shape.)
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "cli.websocket_agents_instances_listener.RowTableKind")]
pub enum RowTableKind {
    MessageQueueText,
    MessageQueueImage,
    MessageQueueAudio,
    MessageQueueVideo,
    MessageQueueFile,
    AssistantResponseRefusal,
    AssistantResponseReasoning,
    AssistantResponseToolCalls,
    AssistantResponseContentText,
    AssistantResponseContentImage,
    AssistantResponseContentAudio,
    AssistantResponseContentVideo,
    AssistantResponseContentFile,
    ToolResponseContentText,
    ToolResponseContentImage,
    ToolResponseContentAudio,
    ToolResponseContentVideo,
    ToolResponseContentFile,
    RequestMessageUserContentText,
    RequestMessageUserContentImage,
    RequestMessageUserContentAudio,
    RequestMessageUserContentVideo,
    RequestMessageUserContentFile,
    RequestMessageAssistantRefusal,
    RequestMessageAssistantReasoning,
    RequestMessageAssistantToolCalls,
    RequestMessageAssistantContentText,
    RequestMessageAssistantContentImage,
    RequestMessageAssistantContentAudio,
    RequestMessageAssistantContentVideo,
    RequestMessageAssistantContentFile,
    RequestMessageToolContentText,
    RequestMessageToolContentImage,
    RequestMessageToolContentAudio,
    RequestMessageToolContentVideo,
    RequestMessageToolContentFile,
    RequestVectorChoiceContentText,
    RequestVectorChoiceContentImage,
    RequestVectorChoiceContentAudio,
    RequestVectorChoiceContentVideo,
    RequestVectorChoiceContentFile,
    ResponseVectorVote,
    ToolResponse,
    RequestMessageTool,
    RequestVectorChoice,
}

/// One row's full current content. Content shapes mirror
/// `agents logs open`'s payloads (`Text`/`Image`/`Audio`/`Video`/`File`),
/// plus the kinds that are inlined here rather than opened by id.
#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.websocket_agents_instances_listener.RowContent")]
pub enum RowContent {
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
    #[schemars(title = "Refusal")]
    Refusal { text: String },
    #[schemars(title = "Reasoning")]
    Reasoning { text: String },
    /// A tool call's full current state — `arguments` is the complete
    /// accumulated string so far (full-value, not a fragment).
    #[schemars(title = "ToolCall")]
    ToolCall {
        tool_call_id: String,
        function_name: String,
        arguments: String,
    },
    /// The per-agent vote: this agent's score for each choice, in
    /// choice order.
    #[schemars(title = "Vote")]
    Vote {
        #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
        #[schemars(with = "Vec<f64>")]
        vote: Vec<rust_decimal::Decimal>,
    },
    /// A HEAD row ([`RowTableKind::ToolResponse`] /
    /// [`RowTableKind::RequestMessageTool`] /
    /// [`RowTableKind::RequestVectorChoice`]): its payload rides the
    /// row's metadata fields (`tool_call_id` / `choice_key`), not the
    /// content.
    #[schemars(title = "Head")]
    Head,
}

/// One conversation row, content inlined. The logs-list part shape
/// (`{id, delivered_at, type}`) with the DB-born `id` replaced by the
/// writer-assigned identity `(table, response_id, row_index,
/// row_sub_index)` and the actual content.
#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.websocket_agents_instances_listener.ConversationRow")]
pub struct ConversationRow {
    pub agent_instance_hierarchy: String,
    /// The enclosing agent completion's id — same semantics as the
    /// logs-list blocks' `response_id`.
    pub response_id: String,
    pub table: RowTableKind,
    /// `objectiveai.messages.row_index` semantics: the message's index
    /// within `messages[]`, the choice index for vector-choice rows,
    /// the `message_queue_contents.id` for notification rows, and `0`
    /// for the (index-less) vote row.
    pub row_index: i64,
    /// `part_index` for content-part rows, `tool_call_index` for
    /// tool-call rows; `None` where the shape has no sub-index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub row_sub_index: Option<i64>,
    /// RFC3339. Live rows: when the writer shipped this value.
    /// Snapshot rows: the row's logged timestamp.
    pub delivered_at: String,
    /// The tool call this row answers — on the two tool HEAD kinds
    /// (live) and on tool-response / request-tool content rows in
    /// snapshots (joined). Block assembly accepts either source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub tool_call_id: Option<String>,
    /// This agent's voting key for the choice — on the
    /// `request_vector_choice` HEAD (live) and on choice content rows
    /// in snapshots (joined).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub choice_key: Option<String>,
    /// Enqueuer AIH — notification (`message_queue_*`) rows only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub sender_agent_instance_hierarchy: Option<String>,
    /// RFC3339 `message_queue.enqueued_at` — notification rows only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub queued_at: Option<String>,
    /// Idempotency key of the consumed queue row, if one was set —
    /// notification rows only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub message_queue_key: Option<String>,
    /// `message_queue.id` of the consumed parent queue row —
    /// notification rows only (the ClientNotification block boundary).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub message_queue_id: Option<i64>,
    pub content: RowContent,
}

/// One event on the `/agents/instances/{*aih}` stream. Two structurally
/// independent concerns ride it:
///
/// - **The conversation**: the DB snapshot replays as `Row` events (in
///   `objectiveai.messages."index"` order), one `Live` marks the
///   snapshot complete, then live `Row` events follow as the
///   conversation occurs.
/// - **The agent's status**: `Agent` carries this agent's list record
///   (active flag + tags + counters), once at connect and on every
///   change.
///
/// Extensibility contract: future variants WILL be added (e.g.
/// client-message delivery notifications) — consumers must skip events
/// they cannot parse.
#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.websocket_agents_instances_listener.AgentInstanceEvent")]
pub enum AgentInstanceEvent {
    /// One conversation row (snapshot replay or live). Replaces any
    /// prior row with the same identity.
    #[schemars(title = "Row")]
    Row { row: ConversationRow },
    /// The snapshot is complete; every following `Row` is live.
    #[schemars(title = "Live")]
    Live,
    /// This agent's list record — the same shape
    /// `/agents/instances/list` tracks (lock-driven `active` flag,
    /// bound tags, counters). A FULL-VALUE upsert: sent once on
    /// connect with the current state, then re-sent whenever it
    /// changes (activation; deactivation — on stream end OR holder
    /// kill; a tag apply/move/removal). Replaces any prior value;
    /// structurally independent of the conversation rows.
    #[schemars(title = "Agent")]
    Agent { agent: AgentRecord },
}

/// One choice of a [`ConversationBlock::VectorRequestChoices`] block:
/// this agent's inline voting `key` plus the choice's content parts in
/// request order.
#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "cli.websocket_agents_instances_listener.ConversationChoice")]
pub struct ConversationChoice {
    /// The prefix-tree voting key this agent assigned to the choice.
    pub key: String,
    pub parts: Vec<ConversationRow>,
}

/// One materialized conversation block — the mirror of
/// `agents logs list`'s `ResponseItem` with content-inlined parts.
/// Produced by the listener's incremental coalescer; blocks appear in
/// conversation order (snapshot `"index"` order, then live write
/// order).
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
        parts: Vec<ConversationRow>,
    },
    /// An `assistant`-role message from the request/task input.
    #[schemars(title = "RequestMessageAssistant")]
    RequestMessageAssistant {
        agent_instance_hierarchy: String,
        response_id: String,
        parts: Vec<ConversationRow>,
    },
    /// A `tool`-role message from the request/task input.
    #[schemars(title = "RequestMessageTool")]
    RequestMessageTool {
        agent_instance_hierarchy: String,
        response_id: String,
        /// The wire tool-call id this request message answers.
        tool_call_id: String,
        parts: Vec<ConversationRow>,
    },
    /// The response choices a vector-completion task voted over.
    #[schemars(title = "VectorRequestChoices")]
    VectorRequestChoices {
        agent_instance_hierarchy: String,
        response_id: String,
        choices: Vec<ConversationChoice>,
    },
    /// The closer for a vector task: this agent's own vote.
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
        response_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        sender_agent_instance_hierarchy: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        queued_at: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        key: Option<String>,
        parts: Vec<ConversationRow>,
    },
    /// The agent's own assistant output.
    #[schemars(title = "AssistantResponse")]
    AssistantResponse {
        agent_instance_hierarchy: String,
        response_id: String,
        parts: Vec<ConversationRow>,
    },
    /// One tool call's response.
    #[schemars(title = "ToolResponse")]
    ToolResponse {
        agent_instance_hierarchy: String,
        response_id: String,
        /// The wire tool-call id this response answers.
        tool_call_id: String,
        parts: Vec<ConversationRow>,
    },
}
