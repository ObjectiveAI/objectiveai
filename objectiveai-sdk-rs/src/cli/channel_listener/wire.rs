//! Wire vocabulary for the daemon's `/channels` endpoint — the
//! DUPLEX CHANNELS surface. A publisher (a trusted `/execute` caller)
//! offers a channel; the first connected SSE client to ACCEPT owns
//! it. Thereafter the two sides exchange messages over an append-only
//! per-channel log (`channels logs request|reply|list|open|subscribe`).
//!
//! `GET /channels` is an SSE stream of [`ChannelEvent`]s. The FIRST
//! frame is always [`ChannelEvent::Connection`], carrying this
//! connection's secret (`S_conn`) — the credential the client
//! presents to accept an offer. The daemon then replays every open
//! OFFER as [`ChannelEvent::Offer`] and sends [`ChannelEvent::Live`]
//! once caught up; live offers, withdrawals, owner-secret deliveries,
//! and closes follow.
//!
//! `POST /channels/{id}/accept` carries a [`ChannelAccept`] body
//! (the caller's `S_conn`). The daemon answers with a
//! [`ChannelAcceptOutcome`] — NEVER the channel secret. On success it
//! PUSHES the owner secret (`S_owner`) back down the accepting
//! connection's SSE as [`ChannelEvent::OwnerSecret`], binding the
//! capability to the actual stream holder even if `S_conn` leaks.

use serde::{Deserialize, Serialize};

use crate::cli::command::AgentArguments;

/// One channel OFFER, as broadcast to every connected channel stream.
/// Carries no secret — the publisher's `S_pub` is returned to the
/// publisher's command, and the owner's `S_owner` is delivered only
/// over the accepting connection's stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.channel_listener.ChannelOffer")]
pub struct ChannelOffer {
    /// The daemon-minted channel id — the accept + log routing key.
    pub channel_id: String,
    /// The PLUGIN that originated the offer — daemon-authored
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
    /// Caller-chosen discriminator (e.g. `"browser.login"`) — how a
    /// user surface decides whether/how to accept the offer.
    pub key: String,
    /// Arbitrary offer payload, opaque to the daemon.
    pub details: serde_json::Value,
}

/// One frame on the `GET /channels` SSE stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.channel_listener.ChannelEvent")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChannelEvent {
    /// This connection's secret (`S_conn`) — ALWAYS the first frame.
    /// Present it to accept an offer.
    #[schemars(title = "Connection")]
    Connection { secret: String },
    /// A channel offer — live broadcast or connect-time replay.
    #[schemars(title = "Offer")]
    Offer { offer: ChannelOffer },
    /// The offer is no longer available (accepted elsewhere, or the
    /// publisher abandoned it). Sent only to connections that saw it.
    #[schemars(title = "OfferWithdrawn")]
    OfferWithdrawn { channel_id: String },
    /// The owner secret (`S_owner`) for a channel THIS connection just
    /// accepted — sent ONLY to the accepting connection, NEVER in the
    /// accept POST response.
    #[schemars(title = "OwnerSecret")]
    OwnerSecret { channel_id: String, secret: String },
    /// An open channel closed (owner dropped / ended): no further
    /// requests or replies are accepted, though the log survives.
    #[schemars(title = "Closed")]
    Closed { channel_id: String },
    /// The connect-time replay is complete — this connection is caught
    /// up. Sent exactly once per connection, right after the replay.
    #[schemars(title = "Live")]
    Live,
}

/// The `POST /channels/{id}/accept` body — the caller's `S_conn`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.channel_listener.ChannelAccept")]
pub struct ChannelAccept {
    /// The accepting connection's secret, from the first
    /// [`ChannelEvent::Connection`] frame.
    pub conn_secret: String,
}

/// The daemon's answer to one `POST /channels/{id}/accept`. Carries
/// NO secret — on success the owner secret arrives over the SSE.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.channel_listener.ChannelAcceptOutcome")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChannelAcceptOutcome {
    /// The offer was accepted: the owner secret is being delivered
    /// over this connection's SSE, and the publisher's command
    /// unblocks.
    #[schemars(title = "Accepted")]
    Accepted,
    /// The offer was already accepted by someone else.
    #[schemars(title = "AlreadyAccepted")]
    AlreadyAccepted,
    /// No open offer with that channel id.
    #[schemars(title = "NotFound")]
    NotFound,
    /// The presented `S_conn` maps to no live connection.
    #[schemars(title = "UnknownConnection")]
    UnknownConnection,
}
