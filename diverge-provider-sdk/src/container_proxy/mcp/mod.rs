//! The `/mcp/*` paths: the container's MCP exchanges, answered by
//! the caller's MCP servers.
//!
//! To the agent inside an agent container, the proxy is a fully
//! compliant MCP server at `/mcp/agent` on the same port. Every
//! exchange the agent asks of it leaves the container on one of five
//! paths — one per exchange — and the caller's servers answer on the
//! far side of the provider.
//!
//! | path                   | exchange           | answered |
//! |------------------------|--------------------|----------|
//! | `/mcp/list-tools`      | [`list_tools`]     | once     |
//! | `/mcp/list-resources`  | [`list_resources`] | once     |
//! | `/mcp/call-tool`       | [`call_tool`]      | once     |
//! | `/mcp/read-resource`   | [`read_resource`]  | once     |
//! | `/mcp/notifications`   | [`notifications`]  | a stream |
//!
//! The five are what an MCP server is once the transport is taken
//! off it: a client POSTs a JSON-RPC message for the first four and
//! opens a stream with a bare `GET` for the fifth. The verb is the
//! whole of the distinction there, and the path is the whole of it
//! here — which is why there is no request enum: a path names one
//! exchange, so both its frames are typed end to end with the
//! shapes [`shared::mcp`](crate::shared::mcp) defines.
//!
//! Every path is an exchange path (see [the module](super)): the
//! container opens a channel with one request, the server answers
//! with the exchange's response — or, on the notification path,
//! one response per notification for as long as the channel lives —
//! and the finish.
//!
//! ```text
//! container → server:  [channel: u8][params JSON…]
//! server → container:  [type: u8][channel: u8][response JSON…]
//! ```
//!
//! # A connection dying does not fail an exchange
//!
//! An exchange is answered when its response has arrived AND its
//! channel has finished. A channel that died before that point left
//! the exchange un-answered, and the container asks it again on the
//! next connection, as a fresh channel. So a caller may legitimately
//! receive the same logical ask twice, and an ask with side effects
//! may be performed twice; that is the chosen trade, because the
//! agent inside is waiting and the alternative is telling it a
//! transport story it can do nothing about. The one non-answer that
//! is not re-asked is the deliberate one: a finish with no response
//! is the far side's statement, not an accident.
//!
//! Each path holds its own connection, its own channels and its own
//! fate; a notification stream dying does not un-answer a tool call
//! in flight on another path.

pub mod call_tool;
pub mod list_resources;
pub mod list_tools;
pub mod notifications;
pub mod read_resource;
