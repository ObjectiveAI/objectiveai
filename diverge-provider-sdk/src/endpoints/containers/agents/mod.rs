//! An agent in a container.
//!
//! The image runs an agentic loop, and the caller drives it over a
//! channel: [`agentic_loop`](crate::shared::containers::agentic_loop)
//! carries the loop's request — a JSON value the image defines — and
//! answers with the loop's chunks;
//! [`schema`](crate::shared::containers::schema) returns what that
//! value may be, so a caller can learn an image's request without
//! knowing the image. The request being a value is what lets one wire
//! carry every agent: what an image accepts is its own to say, and
//! its schema is how it says it.
//!
//! [`agent`] is what that value USED to be — the typed configurations
//! of the agents this crate once named — held for reference, and not
//! on the wire.
//!
//! [`run`] owns the container; [`connect`] joins one.

pub mod agent;
pub mod connect;
pub mod run;
