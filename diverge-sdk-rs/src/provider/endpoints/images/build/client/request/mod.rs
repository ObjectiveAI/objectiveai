//! Image build request data.
//!
//! [`Frame`] is the request — the description, the Containerfile,
//! and the context if any — and [`Context`] is the directory of a volume the
//! build reads from.

mod context;
mod frame;

pub use context::*;
pub use frame::*;
