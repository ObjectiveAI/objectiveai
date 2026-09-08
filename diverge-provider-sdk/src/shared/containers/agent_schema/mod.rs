//! Asking an agent container what its agent may be.
//!
//! The agents family's second exchange, and the one that makes the
//! first usable without knowing the image: the caller opens a channel
//! with [`request::Request`], which carries nothing, and the provider
//! answers with one [`response::Frame`] — the JSON Schema of the
//! [`agent`](crate::shared::containers::agentic_loop::request::Request::agent)
//! value an [`agentic_loop`](crate::shared::containers::agentic_loop)
//! request carries, or an error — then the finish.
//!
//! The schema is the AGENT's, not the request's. The request's other
//! field is a prompt and is this crate's to type; the agent is the
//! image's to define, and this is how the image says what it takes.
//! A schema constrains a document, which is exactly what that value
//! is: one value, sent once. What it cannot say — how the loop's
//! chunks are ordered, when the stream ends — is not its job, and
//! this crate says that in prose as it does everywhere.

pub mod request;
pub mod response;
