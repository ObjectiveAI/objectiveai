//! Editing an agent: its name, its account, its mounts.
//!
//! One request, one answer. A client names an agent of its own and
//! states anew whichever of those it names — each replaced whole,
//! the rest as they are — and the daemon answers that the agent has
//! them, that no agent is the one named, that the agent is active and
//! its mounts were left as they are, that the name is another's, that
//! the account named is none the daemon has, or that it failed, and
//! the scope finishes. These are what changes
//! about an agent after its [`create`](super::create): its image, its
//! limits, its provider and its arguments are for its life.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
