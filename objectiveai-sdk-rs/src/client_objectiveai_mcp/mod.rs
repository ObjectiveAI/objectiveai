//! Client-side ObjectiveAI MCP envelopes — the shapes the API pushes
//! down the reverse-attach channel from #193 (stage 2+).
//!
//! Four surfaces, two pairs:
//!
//! - [`client_request::Request`] — addressed to the client-app layer
//!   (e.g. notify a running agent completion). Carries a correlation `id`.
//! - [`client_response::Response`] — client-app layer's reply to a
//!   `client_request::Request`. Echoes the request's `id`.
//! - [`server_request::Request`] — addressed to the MCP-server layer
//!   (standard MCP `tools/list`, `tools/call`). Carries a correlation `id`.
//! - [`server_response::Response`] — MCP-server layer's reply to a
//!   `server_request::Request`. Echoes the request's `id`.

pub mod client_request;
pub mod client_response;
mod kind;
pub mod retrieve;
pub mod server_request;
pub mod server_response;

pub use kind::McpKind;
