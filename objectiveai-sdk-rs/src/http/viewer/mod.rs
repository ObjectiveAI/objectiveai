//! Viewer HTTP client + request wire shapes.
//!
//! Standalone publisher of streaming-checkpoint events (agent
//! completion / function execution) to a remote viewer's HTTP
//! server. Lives in the `http` module alongside `HttpClient`.

mod client;
mod request;

pub use client::*;
pub use request::*;
