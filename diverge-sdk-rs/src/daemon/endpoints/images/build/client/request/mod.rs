//! Image build request data. What a caller hands the daemon to build
//! an image: [`Frame`], the provider, the Containerfile, a description
//! and the context; and [`Context`], the directory of a volume the
//! build reads from.

mod context;
mod frame;

pub use context::*;
pub use frame::*;
