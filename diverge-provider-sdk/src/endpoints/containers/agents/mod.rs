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
//! to say, and its schema is how it says it.
//!
//! [`agent`] is what that value USED to be — the typed configurations
//! of the agents this crate once named — held for reference, and not
//! on the wire.
//!
//! [`run`] owns the container; [`connect`] joins one.

pub mod agent;
pub mod connect;
pub mod run;
