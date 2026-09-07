//! Asking an agent container what its loop's request may be.
//!
//! The agents family's second exchange, and the one that makes the
//! first usable without knowing the image: the caller opens a channel
//! with [`request::Request`], which carries nothing, and the provider
//! answers with one [`response::Frame`] — the JSON Schema of the value
//! an [`agentic_loop`](crate::shared::containers::agentic_loop)
//! request may carry, or an error — then the finish.
//!
//! A schema constrains a document, which is exactly what the loop's
//! request is: one value, sent once. What it cannot say — how the
//! loop's chunks are ordered, when the stream ends — is not its job,
//! and this crate says that in prose as it does everywhere.

pub mod request;
pub mod response;
