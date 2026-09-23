//! An agent's log: everything its conversation has carried, read.
//!
//! The daemon keeps, for every agent it runs, every chunk the
//! agent's run streamed — the user parts of each message that landed,
//! everything the agent said, its tool calls and their answers, its
//! usage and notifications — and every error the run answered with,
//! each under an id that counts up and the time it was kept. The
//! log outlives the run: it is the daemon's, kept until the agent is
//! deleted.
//!
//! One scope reads it. A client names an agent of its own and says
//! what it wants: a span of ids, a span of time, one kind of item, a
//! jq program to run over what matches, and whether to stay
//! subscribed — every one optional, and a request that says nothing
//! is the whole log. The daemon sends what matches, oldest first,
//! one response each, then finishes; or, subscribed, stays open and
//! sends each item as it lands, and says when the agent goes idle,
//! for as long as the client keeps the scope. An agent that does not
//! exist, or a program that will not compile or fails while it runs,
//! is the scope's error.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
