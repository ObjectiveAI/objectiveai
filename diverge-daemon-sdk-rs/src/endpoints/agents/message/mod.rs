//! Sending an agent a message.
//!
//! A client names an agent of its own and hands the daemon a message
//! for it — MCP content blocks, as an enqueue on the provider's wire
//! carries them — and the scope stays open until the message's fate
//! is known: the daemon answers that the agent took it, that the
//! client cancelled it first, or that it failed, and the scope
//! finishes. While the scope is open the client may open one
//! channel on it, [`cancel`](client::channel_request::Frame::Cancel),
//! to take the message back; nothing answers on that channel, and
//! the scope's own response says whether the cancel was in time.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question and the cancel are in [`client`] — and the daemon
//! answers, so the answer is in [`server`]. Neither side holds both
//! halves of the exchange.

pub mod client;
pub mod server;
