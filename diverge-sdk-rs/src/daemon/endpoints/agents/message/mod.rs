//! Asking an agent something: a chat message, or a structured
//! decision.
//!
//! One request, whose `type` says which of the two things an agent's
//! image may say it does in its `/DIVERGE.md`:
//!
//! - `chat` hands the agent a message — MCP content blocks — which
//!   joins its conversation.
//! - `structured_decisions` hands it a prompt and typed questions,
//!   which it answers with numbers.
//!
//! Both go to the agent in the order the daemon admitted them,
//! whichever kind each is. They are handed over differently: a chat
//! message joins a loop that is running, and a structured decision
//! waits for the agent to be idle, runs alone, and is finished before
//! anything after it starts — see
//! [`enqueue`](crate::shared::containers::enqueue).
//!
//! Unwatched, the request is one question and one answer, open until
//! the fate is known: the agent's program has it, or it failed.
//! Watched, the answer is the part of the agent's log that is the
//! request's: the request as the agent echoed it, and everything the
//! log took after, to the end of that run — see the
//! [response](server::response::Frame). The daemon enqueues each
//! request on the provider's wire under a key of its own minting, and
//! nothing takes one back once sent.
//!
//! One grant covers both: `message`, over agents.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question and the cancel are in [`client`] — and the daemon answers,
//! so the answer is in [`server`].

pub mod client;
pub mod server;
