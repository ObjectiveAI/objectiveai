//! HTTP responses the calling client's `McpHandler` produces in
//! reply to a [`super::server_request::Request`]. The API translates
//! the response into a plain HTTP response sent back to the proxy
//! that originally hit `/objectiveai-mcp/{session_id}`.

mod response;
pub use response::*;
mod payload;
pub use payload::*;
