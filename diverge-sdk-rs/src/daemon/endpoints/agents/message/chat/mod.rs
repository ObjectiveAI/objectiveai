//! Sending an agent a chat message.
//!
//! A client names an agent of its own and hands the daemon a message
//! for it — MCP content blocks, as an enqueue on the provider's wire
//! carries them — and the scope stays open until the message's fate
//! is known: the daemon answers that the agent's program has it, or
//! that it failed, and the scope finishes. One question, one answer:
//! the client opens no channel on the scope, and nothing takes a
//! message back once sent. The daemon enqueues each message on the
//! provider's wire under a key of its own minting, one per message.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
