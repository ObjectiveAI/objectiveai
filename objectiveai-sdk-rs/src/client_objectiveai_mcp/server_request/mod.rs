//! Requests the API ships to the calling client's `McpHandler` over
//! the reverse channel. The CLI's `ConduitMcpHandler` is the
//! canonical implementation: a pure router forwarding each op to the
//! laboratory host serving the addressed MCP container.
//!
//! Each request carries a server-minted `id` that the client echoes
//! in the matching [`super::server_response::Response`] so the
//! server can correlate replies to in-flight requests.

mod request;
pub use request::*;
mod payload;
pub use payload::*;
