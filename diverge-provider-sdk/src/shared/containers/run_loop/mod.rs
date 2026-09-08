//! Running the loop in an agent container.
//!
//! The agents family's own exchange. The container was made with a
//! prompt and an agent on its
//! [`request`](crate::endpoints::containers::agents::run::client::request::Frame),
//! so the channel that starts the loop has nothing left to say: the
//! caller's [`RunLoop`](crate::endpoints::containers::agents::run::client::channel_request::Frame::RunLoop)
//! carries no payload at all — a direction that carries nothing has
//! no request here, as everywhere in this crate — and the provider
//! answers with the loop as it happens: one
//! [`response::Frame`] per chunk, then the finish, or an
//! [`Error`](response::Frame::Error) when there is no loop to report
//! on. A container runs one loop; what a second opening means while
//! one runs, or after one ended, is the image's to define.
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

pub mod response;
