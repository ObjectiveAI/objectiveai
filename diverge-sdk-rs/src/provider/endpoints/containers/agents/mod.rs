//! An agent in a container.
//!
//! The image runs an agent, and the caller talks to it. The agent is
//! what the image makes of the
//! [`arguments`](crate::shared::containers::request::Container::arguments)
//! on the request that made the container, fixed for its life; what
//! the agent says rides the scope's own main stream, chunk by chunk,
//! after the id — see [`run::server::response`]. The caller speaks to it with
//! [`enqueue`](crate::shared::containers::enqueue): a message with no
//! loop running starts one, and a message while one runs joins its
//! queue, taken at a seam of the agent's choosing;
//! [`dequeue`](crate::shared::containers::dequeue) withdraws
//! everything not yet taken; neither touches the turn being run.
//! [`schema`](crate::shared::containers::schema) returns what the
//! arguments may be, so a caller can learn an image's agent without
//! knowing the image. The arguments being a value is what lets one
//! wire carry every agent: what an image accepts is its own to say,
//! and its schema is how it says it.
//!
//! [`run`] owns the container, and there is no connect: an agent
//! container is its runner's alone. A conversation has one caller —
//! the one that sends its messages and reads its chunks — and a
//! second scope on it would be a second party to a conversation that
//! has one side. What a tool container's connect is for, joining a
//! server somebody else runs, has no counterpart here.

pub mod run;
