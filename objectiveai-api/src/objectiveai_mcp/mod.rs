//! Reverse-attach plumbing for the API's MCP forward path.
//!
//! Each CLI WS upgrade owns a [`registry::ReverseChannel`] (its sink +
//! pending-request slot); [`send::send_server_request`] is the API-side
//! write primitive that forwards a `server_request` over that WS and
//! parks an await for the matching `server_response`. The per-request
//! proxy speaks the reverse-channel protocol directly over the same WS;
//! there is no longer a loopback HTTP MCP server in front of it.

mod registry;
mod send;

pub use registry::{
    PendingRequests, ReverseAttachGuard, ReverseAttachHandle, ReverseChannel,
    SharedSink, new_pending_requests,
};
pub use send::send_server_request;
