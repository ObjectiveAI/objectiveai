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
//! The chunks are typed, and they are the one thing a container says
//! that this crate defines: content, reasoning, tool calls, tool
//! results, usage, notifications — MCP's own content vocabulary,
//! flattened, so what a model produces and what a tool returns need
//! no translation between them. See [`response::AgenticLoopChunk`].

pub mod request;
pub mod response;
