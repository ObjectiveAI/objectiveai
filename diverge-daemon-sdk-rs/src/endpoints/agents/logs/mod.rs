//! An agent's log: everything its conversation has carried.
//!
//! The daemon keeps, for every agent it runs, every chunk the
//! agent's run streamed — the user parts of each message that landed,
//! everything the agent said, its tool calls and their answers, its
//! usage and notifications — and every error the run answered with,
//! each under an id that counts up and the time it was kept. The
//! log outlives the run: it is the daemon's, kept until the agent is
//! deleted.
//!
//! [`query`] reads it from an id on.

pub mod query;
