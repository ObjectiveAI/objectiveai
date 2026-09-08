//! An agent in a container.
//!
//! The image runs an agentic loop, and the caller drives it over a
//! channel: [`agentic_loop`](crate::shared::containers::agentic_loop)
//! carries the loop's request — a prompt, and an agent as a JSON
//! value the image defines — and answers with the loop's chunks;
//! [`agent_schema`](crate::shared::containers::agent_schema) returns
//! what that agent value may be, so a caller can learn an image's
//! agent without knowing the image. The agent being a value is what
//! lets one wire carry every agent: what an image accepts is its own
//! to say, and its schema is how it says it. A running loop has a
//! queue, and [`enqueue`](crate::shared::containers::enqueue) and
//! [`dequeue`](crate::shared::containers::dequeue) are a caller's two
//! verbs against it — a message for the conversation in flight, or
//! everything not yet taken withdrawn — neither touching the turn
//! being run.
//!
//! [`agent`] is what that value USED to be — the typed configurations
//! of the agents this crate once named — held for reference, and not
//! on the wire.
//!
//! [`run`] owns the container, and there is no connect: an agent
//! container is its runner's alone. A loop has one caller — the one
//! that gave it its prompt and reads its chunks — and a second scope
//! on it would be a second party to a conversation that has one
//! side. What a tool container's connect is for, joining a server
//! somebody else runs, has no counterpart here.

pub mod agent;
pub mod run;
