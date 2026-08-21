//! The server side of a version request: what a provider sends.
//!
//! [`response`] is the whole of it. A provider answers and is done; it
//! opens no channels of its own for a question this small, so there is
//! no `request` here the way there is on the agentic loop's server
//! side.

pub mod response;
