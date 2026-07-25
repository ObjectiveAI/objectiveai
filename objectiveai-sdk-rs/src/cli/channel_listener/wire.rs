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
//! `POST /channels/{id}/accept` (no body) is the ACCEPT: first-wins
//! over a pending offer, answering `200` with [`ChannelAccepted`] —
//! the owner secret (`S_owner`) — or `404` (unknown/withdrawn) /
//! `409` (already accepted). The daemon does NO liveness tracking:
//! a channel stays open until someone runs `channels close` with
//! either of its secrets (terminal; any blocked `channels logs
//! subscribe` unblocks with `channel_closed`).
//!
//! Secret flow: `S_pub` is minted at offer time and returned to the
//! publisher's command; `S_owner` is minted at accept and returned in
//! the accept response. Both are bearer capabilities for the
//! per-channel log commands.

use serde::{Deserialize, Serialize};

use crate::identity::Identity;

/// One channel OFFER, as broadcast to every connected channel stream.
/// Carries no secret — the publisher's `S_pub` is returned to the
/// publisher's command, and the owner's `S_owner` is returned by the
/// accept POST.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.channel_listener.ChannelOffer")]
pub struct ChannelOffer {
    /// The daemon-minted channel id — the accept + log routing key.
    pub channel_id: String,
    /// The originating caller's identity, FLATTENED to top-level
    /// fields (the flat-identity wire convention). Its plugin trio is
    /// the PUBLISHING plugin — daemon-authored (unspoofable; stamped
    /// by `plugins run`), absent when the caller wasn't a plugin.
    #[serde(flatten)]
    pub identity: Identity,
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

/// The `POST /channels/{id}/accept` success body: the owner secret
/// (`S_owner`) — the per-channel capability for `channels logs
/// reply|list|open|subscribe` and `channels close`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.channel_listener.ChannelAccepted")]
pub struct ChannelAccepted {
    pub secret: String,
}
