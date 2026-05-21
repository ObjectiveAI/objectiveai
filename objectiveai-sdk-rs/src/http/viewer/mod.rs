//! Viewer HTTP client + request wire shapes.
//!
//! Standalone publisher of streaming-checkpoint events (agent
//! completion / function execution / function invention recursive /
//! laboratory execution) to a remote viewer's HTTP server. Lives in
//! the `http` module alongside `HttpClient`. The api server's
//! `objectiveai_api::viewer::Client<CTXEXT>` is a thin context-aware
//! wrapper over this client.

mod client;
mod request;

pub use client::*;
pub use request::*;
