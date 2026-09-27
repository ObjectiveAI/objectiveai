//! Deleting a tool by name.
//!
//! One request, one answer. A client names a tool of its own; the
//! daemon answers that the tool is deleted, that no tool has that
//! name, that the tool is attached to an agent and was left as it is,
//! or that it failed, and the scope finishes. A deleted tool's name
//! is free for a [`create`](super::create). A tool attached nowhere
//! runs nowhere, so a delete stops nothing.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
