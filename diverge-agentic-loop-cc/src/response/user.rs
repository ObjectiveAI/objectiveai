//! The `user` records: what went back to the model.

use serde::{Deserialize, Serialize};

use super::message;

/// A `type: "user"` record: a user turn or a tool result, echoed to
/// stdout — one content block per record, like the assistant's.
///
/// # One struct for the schema's two
///
/// The source defines `SDKUserMessage` and `SDKUserMessageReplay` as
/// separate schemas differing only in obligation: a replay requires
/// `uuid`, `session_id` and `isReplay: true`, the plain one makes the
/// ids optional and has no flag. One struct with those fields
/// optional covers both arms; [`is_replay`](Self::is_replay) present
/// and true is what a replay is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    /// Always `user`.
    pub r#type: UserType,
    /// The message params: role and content, the request vocabulary.
    pub message: message::UserMessage,
    /// The spawning tool call, for a subagent's record; `null` on
    /// the main thread.
    pub parent_tool_use_id: Option<String>,
    /// Set on messages Claude Code made up itself — meta turns and
    /// transcript-only annotations.
    #[serde(
        rename = "isSynthetic",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_synthetic: Option<bool>,
    /// The tool's result in Claude Code's own richer form, beside
    /// the API-shaped block. `unknown` in the schema, and kept so.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_result: Option<serde_json::Value>,
    /// Queueing priority, when the message was queued.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,
    /// When the message was created on the originating process, ISO
    /// 8601; older emitters omit it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    /// Marks an echo of something already said — a resumed history
    /// replay or an acknowledgement — rather than a new turn.
    #[serde(
        rename = "isReplay",
        skip_serializing_if = "Option::is_none"
    )]
    pub is_replay: Option<bool>,
    /// The record's own id; optional on the plain arm.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    /// The session; optional on the plain arm.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// A queued message's priority.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    /// Interrupt for it.
    Now,
    /// After the current step.
    Next,
    /// Whenever.
    Later,
}

/// The `user` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum UserType {
    /// The only value.
    #[default]
    User,
}
