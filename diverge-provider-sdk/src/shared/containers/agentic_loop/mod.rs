//! Running an agentic loop in an agent container.
//!
//! The agents family's own exchange. The caller opens a channel with
//! [`request::Request`] — a prompt, and an agent as a JSON value the
//! IMAGE defines, since what an agent is is the image's business and
//! one wire has to carry every agent — and the provider answers with
//! the loop as it happens: one [`response::Frame`] per chunk, then
//! the finish, or an [`Error`](response::Frame::Error) when there is
//! no loop to report on. What the agent value may be is asked over
//! [`agent_schema`](crate::shared::containers::agent_schema).
//!
//! A running loop has a QUEUE, and a caller has two verbs against
//! it: [`enqueue`](crate::shared::containers::enqueue) puts a message
//! in, [`dequeue`](crate::shared::containers::dequeue) clears whatever
//! has not yet been taken. Neither touches the turn in flight — the
//! agent takes a message at a seam of its own choosing, and marks the
//! delivery in this stream with a
//! [`UserChunk`](response::UserChunk) carrying it verbatim.
//!
//! The chunks are typed, and they are the one thing a container says
//! that this crate defines: content, reasoning, tool calls, tool
//! results, usage, notifications — MCP's own content vocabulary,
//! flattened, so what a model produces and what a tool returns need
//! no translation between them. See [`response::AgenticLoopChunk`].

pub mod request;
pub mod response;
