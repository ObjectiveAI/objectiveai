//! Running an agentic loop in an agent container.
//!
//! The agents family's own exchange. The caller opens a channel with
//! [`request::Request`] — a JSON value the IMAGE defines, since what a
//! loop takes is the agent's business and one wire has to carry every
//! agent — and the provider answers with the loop as it happens: one
//! [`response::Frame`] per chunk, then the finish, or an
//! [`Error`](response::Frame::Error) when there is no loop to report
//! on. What the value may be is asked over
//! [`schema`](crate::shared::containers::schema).
//!
//! The chunks are typed, and they are the one thing a container says
//! that this crate defines: content, reasoning, tool calls, tool
//! results, usage, notifications — MCP's own content vocabulary,
//! flattened, so what a model produces and what a tool returns need
//! no translation between them. See [`response::AgenticLoopChunk`].

pub mod request;
pub mod response;
