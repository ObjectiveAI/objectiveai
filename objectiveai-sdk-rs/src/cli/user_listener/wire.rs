//! Wire vocabulary for the daemon's `/user` channel — the USER
//! REQUESTS surface: plugins (or any command caller) broadcast a
//! request to every connected user stream; the first ACCEPTED reply
//! wins.
//!
//! `GET /user` is an SSE stream of [`UserEvent`]s. On connect the
//! daemon replays every PENDING (unsettled) request as a
//! [`UserEvent::Request`]; live requests, settlements, and timeouts
//! follow. A settled or timed-out request is never replayed — and its
//! [`UserEvent::Settled`] / [`UserEvent::TimedOut`] notice goes only
//! to connections that saw the request.
//!
//! `POST /user/{id}/reply` carries a [`UserReply`] body; the
//! replier's identity rides the standard `X-OBJECTIVEAI-*` request
//! headers (NOT the body). The daemon answers with a
//! [`UserReplyOutcome`] either way (HTTP 200 accepted / 422 rejected
//! by the validator / 409 already settled / 404 unknown).

use serde::{Deserialize, Serialize};

use crate::cli::command::AgentArguments;

/// One outbound user request, as broadcast to every connected user
/// stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.user_listener.UserRequest")]
pub struct UserRequest {
    /// The daemon-minted request id — the reply routing key.
    pub id: String,
    /// The PLUGIN that originated the request — daemon-authored
    /// (unspoofable; stamped by `plugins run`), absent when the
    /// caller wasn't a plugin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub plugin_owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub plugin_repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub plugin_version: Option<String>,
    /// The originating caller's agent identity, from its scope.
    pub agent_arguments: AgentArguments,
    /// Caller-chosen discriminator (e.g. `"AskUserQuestion"`) — how a
    /// user surface decides what UI the `details` drive.
    pub key: String,
    /// Arbitrary request payload, opaque to the daemon.
    pub details: serde_json::Value,
}

/// One frame on the `GET /user` SSE stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.user_listener.UserEvent")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserEvent {
    /// A pending request — live broadcast or connect-time replay.
    #[schemars(title = "Request")]
    Request { request: UserRequest },
    /// The request was settled: `identity` is the winning replier.
    /// Sent only to connections that saw the request; no further
    /// replies are possible.
    #[schemars(title = "Settled")]
    Settled { id: String, identity: AgentArguments },
    /// The request ended without an accepted reply (the originating
    /// command timed out or was cancelled). Sent only to connections
    /// that saw the request.
    #[schemars(title = "TimedOut")]
    TimedOut { id: String },
    /// The connect-time replay is complete — this connection is
    /// caught up (everything pending at connect has been delivered).
    /// Sent exactly once per connection, right after the replay.
    #[schemars(title = "Live")]
    Live,
}

/// The `POST /user/{id}/reply` body. The replier's identity rides the
/// `X-OBJECTIVEAI-*` request headers, not this body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.user_listener.UserReply")]
pub struct UserReply {
    /// The reply payload, opaque to the daemon (the originating
    /// command's optional python validator is the only inspector).
    pub reply: serde_json::Value,
}

/// The daemon's answer to one `POST /user/{id}/reply`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.user_listener.UserReplyOutcome")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserReplyOutcome {
    /// This reply WON: it was accepted (validator included, when one
    /// was set) and unblocks the originating command.
    #[schemars(title = "Accepted")]
    Accepted,
    /// The originating command's python validator refused this reply
    /// — the request is STILL PENDING; the same or another connection
    /// may reply again.
    #[schemars(title = "Rejected")]
    Rejected { message: String },
    /// Another reply already won — this one can no longer be
    /// accepted.
    #[schemars(title = "Settled")]
    Settled,
    /// No pending request with that id (never existed, or already
    /// ended).
    #[schemars(title = "NotFound")]
    NotFound,
}
