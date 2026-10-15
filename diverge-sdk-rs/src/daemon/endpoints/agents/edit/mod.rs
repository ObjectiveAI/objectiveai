//! Editing an agent: a patch to its name and its mounts.
//!
//! One request, one answer. A client names an agent of its own and a
//! patch — see [`patch`](crate::daemon::patch) — to the agent's
//! editable members; the daemon applies it, whole or not at all, and
//! answers that the agent is as the result states, that no agent is
//! the one named, that the agent is active and its mounts were left as
//! they are, that the name is another's, or that it failed, and the
//! scope finishes. What changes about an agent after its
//! [`create`](super::create) is its name and its mounts: its image, its
//! limits, its provider and its arguments are for its life.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
