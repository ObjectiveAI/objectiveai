//! Running the loop in an agent container.
//!
//! The agents family's own exchange. The container was made with an
//! agent on its
//! [`request`](crate::endpoints::containers::agents::run::client::request::Frame),
//! fixed for its life; what each loop is asked is the loop's own, so
//! the caller's [`AgentRun`](crate::endpoints::containers::agents::run::client::channel_request::Frame::AgentRun)
//! carries a [`request::Request`] — the prompt — and the provider
//! answers with the loop as it happens: one [`response::Frame`] per
//! chunk, then the finish, or an [`Error`](response::Frame::Error)
//! when there is no loop to report on. A container runs loops one
//! after another, each resuming the conversation the last one left,
//! and one at a time: a second opening while one runs is the image's
//! refusal, as an `Error`.
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
