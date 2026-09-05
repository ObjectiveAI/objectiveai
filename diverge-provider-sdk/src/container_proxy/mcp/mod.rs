//! MCP: the container's exchanges, answered by the caller's MCP
//! servers, and their notifications on request.
//!
//! To the agent inside an agent container, the proxy is a fully
//! compliant MCP server at `/mcp/agent` on the same port. Every
//! exchange the agent asks of it leaves the container as one of five
//! asks on `/requests`, and the caller's servers answer on the ask's
//! own path.
//!
//! | ask on `/requests` | kind | answered on | with one message of |
//! |--------------------|------|-------------|---------------------|
//! | [`McpListTools`](crate::container_proxy::requests::request::Request::McpListTools) | `0` | `/mcp/list-tools/{channel}` | [`list_tools::response::Frame`] |
//! | [`McpListResources`](crate::container_proxy::requests::request::Request::McpListResources) | `1` | `/mcp/list-resources/{channel}` | [`list_resources::response::Frame`] |
//! | [`McpCallTool`](crate::container_proxy::requests::request::Request::McpCallTool) | `2` | `/mcp/call-tool/{channel}` | [`call_tool::response::Frame`] |
//! | [`McpReadResource`](crate::container_proxy::requests::request::Request::McpReadResource) | `3` | `/mcp/read-resource/{channel}` | [`read_resource::response::Frame`] |
//! | [`McpNotifications`](crate::container_proxy::requests::request::Request::McpNotifications) | `4` | `/mcp/notifications/{channel}` | [`notifications::response::Frame`], one per notification |
//!
//! The five are what an MCP server is once the transport is taken
//! off it: a client POSTs a JSON-RPC message for the first four and
//! opens a stream with a bare `GET` for the fifth. Each of the first
//! four's answer path carries exactly one message — the exchange's
//! own response type from [`shared::mcp`](crate::shared::mcp), raw,
//! result or error inside it — and then the close. The fifth's
//! carries one message per notification for as long as the caller's
//! servers have them, an
//! [`Error`](crate::shared::mcp::notifications::response::Frame::Error)
//! being the last, and then the close; and it is opened only because
//! the container asked — a container that never asks never hears a
//! notification. Nothing here defines a type of its own, because a
//! newtype over a type used raw would be ceremony.
//!
//! # A connection dying does not fail an exchange
//!
//! An exchange is answered when its one message has arrived AND its
//! path has closed cleanly. One that died before that point — its
//! `/requests` connection went before the path opened, or the path
//! ended abruptly — left the exchange un-answered, and the container
//! asks it again on the next `/requests` connection, as a fresh
//! channel. So a caller may legitimately receive the same logical
//! ask twice, and an ask with side effects may be performed twice;
//! that is the chosen trade, because the agent inside is waiting and
//! the alternative is telling it a transport story it can do nothing
//! about. The one non-answer that is not re-asked is the deliberate
//! one: a path opened and closed cleanly with no message is the far
//! side's statement, not an accident. A notification stream that
//! died is re-asked the same way, and what it missed in between is
//! missed — notifications are not replayed.

pub mod call_tool;
pub mod list_resources;
pub mod list_tools;
pub mod notifications;
pub mod read_resource;
