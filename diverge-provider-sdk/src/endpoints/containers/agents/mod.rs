//! An agent in a container.
//!
//! The image runs agentic loops. The agent — a JSON value the image
//! defines — is on the [`request`](run::client::request::Frame) that
//! makes the container, fixed for its life, and the caller runs each
//! loop over a channel, [`run_loop`](crate::shared::containers::run_loop),
//! that carries the loop's prompt and answers with its chunks;
//! [`agent_schema`](crate::shared::containers::agent_schema) returns
//! what the agent value may be, so a caller can learn an image's
//! agent without knowing the image. The agent being a value is what
//! lets one wire carry every agent: what an image accepts is its own
//! to say, and its schema is how it says it. A running loop has a
//! queue, and [`enqueue`](crate::shared::containers::enqueue) and
//! [`dequeue`](crate::shared::containers::dequeue) are a caller's two
//! verbs against it — a message for the conversation in flight, or
//! everything not yet taken withdrawn — neither touching the turn
//! being run.
//!
//! [`run`] owns the container, and there is no connect: an agent
//! container is its runner's alone. A loop has one caller — the one
//! that gave it its prompt and reads its chunks — and a second scope
//! on it would be a second party to a conversation that has one
//! side. What a tool container's connect is for, joining a server
//! somebody else runs, has no counterpart here.

pub mod run;
