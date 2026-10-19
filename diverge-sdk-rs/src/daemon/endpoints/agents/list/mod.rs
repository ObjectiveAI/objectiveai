//! Listing a caller's agents.
//!
//! A client asks for its agents, narrowed, and the daemon sends every
//! one it holds under the caller's identity that the request's filter
//! lets through — by name, by template, by creator, by activity, by
//! tags, all or any, by when it was created — one response each, oldest
//! created first, then the word that the list is whole, and keeps the
//! scope open: each agent added, changed or removed, as the records,
//! the attachments and the runs change, until the client cancels, the
//! one channel it opens on the scope. Each is what the agent is called,
//! what template it was made from and its number among the agents ever
//! made from it, who made it, whether it is active now, when its
//! activity last changed and where it ran, how long its log is, which
//! tools are attached to it, and its tags. The log's length is as of
//! the agent's last change of state — a run starting or ending, a loop
//! beginning or ending — and not of its last line: a line appended
//! changes nothing here, and a caller that wants the log as it grows
//! reads its [`logs`](super::logs). A count keeps the list to the
//! first that many that match. A request that says nothing is every
//! agent. A caller with no agent that matches is told the list is
//! whole at once, and watched.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
