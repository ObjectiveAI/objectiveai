//! Wire vocabulary for the daemon's `/channels` endpoints — the
//! DUPLEX CHANNELS surface. A publisher (a trusted `/execute` caller)
//! offers a channel; the first client to OPEN the channel's stream
//! ACCEPTS and owns it. Thereafter the two sides exchange messages
//! over an append-only per-channel log
//! (`channels logs request|reply|list|open|subscribe`).
//!
//! `GET /channels` is an SSE stream of [`ChannelEvent`]s — the OFFER
//! lifecycle and nothing else: the daemon replays every open offer as
//! [`ChannelEvent::Offer`], sends [`ChannelEvent::Live`] once caught
//! up, then live offers and withdrawals follow. No secrets ride this
//! stream.
//!
//! `GET /channels/{id}` is the PER-CHANNEL stream, an SSE of
//! [`ChannelStreamEvent`]s, in one of two modes:
//! - **Accept** — no `X-OBJECTIVEAI-CHANNEL-SECRET` header. Opening
//!   the stream of a PENDING offer IS the accept (first-wins). The
//!   first frame is [`ChannelStreamEvent::Secret`] carrying the owner
//!   secret (`S_owner`), and the stream is the channel's LIVENESS
//!   ANCHOR: when it drops, the channel closes (terminal).
//! - **Observer** — the header carries a channel secret (`S_pub` or
//!   `S_owner`). The stream is silent until the channel closes, then
//!   delivers one [`ChannelStreamEvent::Closed`] and ends. Observer
//!   drops close nothing.
//!
//! Secret flow: `S_pub` is minted at offer time and returned to the
//! publisher's command; `S_owner` is minted at accept and delivered as
//! the accepting stream's first frame — the capability and the
//! channel's life are bound to the same connection by construction.

use serde::{Deserialize, Serialize};

use crate::cli::command::AgentArguments;

/// One channel OFFER, as broadcast to every connected channel stream.
/// Carries no secret — the publisher's `S_pub` is returned to the
/// publisher's command, and the owner's `S_owner` is delivered only
/// over the accepting per-channel stream.
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
    pub plugin_name: Option<String>,
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
    /// Human-readable offer message, opaque to the daemon.
    pub message: String,
}

/// One frame on the `GET /channels` SSE stream — the offer lifecycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.channel_listener.ChannelEvent")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChannelEvent {
    /// A channel offer — live broadcast or connect-time replay.
    #[schemars(title = "Offer")]
    Offer { offer: ChannelOffer },
    /// The offer is no longer available (accepted elsewhere, or the
    /// publisher abandoned it). Sent only to connections that saw it.
    #[schemars(title = "OfferWithdrawn")]
    OfferWithdrawn { channel_id: String },
    /// The connect-time replay is complete — this connection is caught
    /// up. Sent exactly once per connection, right after the replay.
    #[schemars(title = "Live")]
    Live,
}

/// One frame on the per-channel `GET /channels/{id}` SSE stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.channel_listener.ChannelStreamEvent")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChannelStreamEvent {
    /// The owner secret (`S_owner`) — ALWAYS the first frame of an
    /// accept-mode open, never sent on an observer stream. The
    /// per-channel capability for `channels logs reply|list|open|
    /// subscribe`.
    #[schemars(title = "Secret")]
    Secret { secret: String },
    /// The channel closed (terminal): no further requests or replies
    /// are accepted, though the log survives. The stream's last frame.
    #[schemars(title = "Closed")]
    Closed,
}
