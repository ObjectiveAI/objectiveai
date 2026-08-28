//! What the harness writes to Claude Code's stdin.
//!
//! The mirror of [`response`](crate::response), pointed the other
//! way and holding only what this container actually says — the
//! stream-json INPUT vocabulary is far wider (the source's whole
//! stdin union), but a shape nothing sends is a shape nobody has had
//! to be right about. Serialize-only, the way `response` is
//! deserialize-only: each type becomes one NDJSON line and nothing
//! here ever reads one back.

use serde::Serialize;

/// A user message for the running session: an enqueued prompt, or
/// the run's first one — stream-json input mode's way of delivering
/// the print prompt is the same line an enqueue writes.
///
/// The [`uuid`](Self::uuid) is the correlation key: with
/// `--replay-user-messages` on, Claude Code echoes the message back
/// on stdout at the moment it enters the conversation, carrying this
/// uuid — which is how the drain resolves the message's fate as
/// delivered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserMessage {
    /// Always `user`.
    pub r#type: UserMessageType,
    /// The message body.
    pub message: UserMessageBody,
    /// The correlation key the replay echo carries back.
    pub uuid: String,
}

/// A [`UserMessage`]'s body: role and content, nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserMessageBody {
    /// Always `user`.
    pub role: UserRole,
    /// The message's text. A string, deliberately: the enqueue
    /// vocabulary is text, and the rich-content prompt arrives with
    /// the conversion work or not at all.
    pub content: String,
}

/// The `user` type literal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UserMessageType {
    /// The only value.
    #[default]
    User,
}

/// The `user` role literal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    /// The only value.
    #[default]
    User,
}

/// A control request: the wrapper every control ask rides in.
///
/// One inner kind, because this container asks Claude Code for
/// exactly one thing: dropping a queued message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ControlRequest {
    /// Always `control_request`.
    pub r#type: ControlRequestType,
    /// The ask's id. Claude Code answers on stdout quoting it, and
    /// the dequeue that wrote this waits for that answer — the id is
    /// how the reply and the ask find each other.
    pub request_id: String,
    /// The ask itself.
    pub request: CancelAsyncMessage,
}

/// The `control_request` type literal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlRequestType {
    /// The only value.
    #[default]
    ControlRequest,
}

/// Drop one queued message, by the uuid its enqueue minted — the
/// source's `cancel_async_message`: "Drops a pending async user
/// message from the command queue by uuid. No-op if already dequeued
/// for execution."
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CancelAsyncMessage {
    /// Always `cancel_async_message`.
    pub subtype: CancelAsyncMessageSubtype,
    /// The queued message to drop.
    pub message_uuid: String,
}

/// The `cancel_async_message` subtype literal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CancelAsyncMessageSubtype {
    /// The only value.
    #[default]
    CancelAsyncMessage,
}
