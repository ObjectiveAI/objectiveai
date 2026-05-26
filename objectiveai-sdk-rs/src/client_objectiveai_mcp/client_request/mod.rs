//! Requests addressed to the client-app layer of the local
//! objectiveai-mcp — non-MCP-protocol pushes (e.g. surface a user
//! message into a running agent completion).
//!
//! Each request carries a client-minted `id` that the server echoes
//! in the matching [`super::client_response::Response`] so the client
//! can correlate replies to in-flight requests.

mod request;
pub use request::*;
mod payload;
pub use payload::*;
