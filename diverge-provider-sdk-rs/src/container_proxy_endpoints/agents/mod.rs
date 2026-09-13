//! An agent container, from the proxy's side.
//!
//! The image runs agentic loops, and the server reaches them through
//! the proxy: the agent is registered once, a loop runs over a channel
//! carrying its prompt and answering with its chunks, the schema says
//! what the agent may be, and enqueue and dequeue work the running
//! loop's queue — the same exchanges a caller opens on the provider,
//! carried the last hop. [`begin`] is the one scope: it opens the
//! connection's work, and every one of those is a channel on it.

pub mod begin;
