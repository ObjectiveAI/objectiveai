//! The server side of the agentic loop: what a provider sends.
//!
//! [`request`] is what it opens channels of its own to ask for.
//! [`response`] is the chunks of its answer, and [`ResponseFrame`] is
//! what carries them. Both happen inside the scope the client's
//! request opened — a server never initiates one.

pub mod request;
pub mod response;

mod response_frame;

pub use response_frame::*;
