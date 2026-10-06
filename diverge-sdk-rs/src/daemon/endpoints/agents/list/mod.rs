//! Listing a caller's agents.
//!
//! A client asks for its agents, narrowed, and the daemon sends every
//! one it holds under the caller's identity that the request's filter
//! lets through — by name, by template, by creator, by activity, by
//! tags, all or any, by when it was created — one response each, oldest
//! created first, and finishes: what each is called, what template it
//! was made from and its number among the agents ever made from it, who
//! made it, whether it is active now, when its activity last changed
//! and where it ran, how long its log is, which tools are attached to
//! it, and its tags. A count caps what comes back. A request that says
//! nothing is every agent. A caller with no agent that matches sees the
//! finish and nothing before it. The daemon does not stay open; a
//! caller that wants to know when an agent's activity changes reads its
//! [`logs`](super::logs).
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
