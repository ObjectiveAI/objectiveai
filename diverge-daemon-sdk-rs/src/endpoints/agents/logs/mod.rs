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
//! what it wants: a span of indexes, a span of time, one kind of
//! item, a jq program to run over what matches, how many values at
//! most, and whether to watch — every one optional, and a request
//! that says nothing is the whole log. The daemon runs the whole
//! filter over the log as it stands, oldest first, one response per
//! value, and finishes; or, watching, goes on to run the same filter
//! over each item as it lands, and stays open until the filter can
//! never match again — the log has reached the request's last
//! index, or the clock its last time — or the count is met, or the
//! client cancels, or the agent is deleted. A filter with no last
//! index, no last time and no count can always match again, and a
//! watch on it never ends on its own. An agent that does not exist,
//! or a program that will not compile or fails while it runs, is the
//! scope's error.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question and the cancel are in [`client`] — and the daemon
//! answers, so the answer is in [`server`]. Neither side holds both
//! halves of the exchange.

pub mod client;
pub mod server;
