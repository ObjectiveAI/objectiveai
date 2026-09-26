//! An agent container, from the proxy's side.
//!
//! The image runs an agent, and the server reaches it through the
//! proxy: the arguments ride the begin, what the agent says rides the
//! begin's own main stream, chunk by chunk, an enqueue sends it a
//! message — starting a loop when none runs, queueing one when it
//! does — a dequeue clears what is waiting, and the schema says what
//! the arguments may be: the same exchanges a caller opens on the
//! provider, carried the last hop. [`begin`] is the one scope: it opens the connection's
//! work, and every one of those is a channel on it.

pub mod begin;
