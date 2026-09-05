//! MCP: the container's exchanges, answered by the caller's MCP
//! servers, and their notifications pushed back.
//!
//! To the agent inside an agent container, the proxy is a fully
//! compliant MCP server at `/mcp/agent` on the same port. Every
//! exchange the agent asks of it leaves the container as one of four
//! asks on `/requests`, and the caller's servers answer on the ask's
//! own path; what they say on their own account is pushed on a
//! stream path of its own.
//!
//! | ask on `/requests` | kind | answered on | with one message of |
//! |--------------------|------|-------------|---------------------|
//! | [`McpListTools`](crate::container_proxy::requests::Request::McpListTools) | `0` | `/mcp/list-tools/{channel}` | [`list_tools::response::Frame`](crate::shared::mcp::list_tools::response::Frame) |
//! | [`McpListResources`](crate::container_proxy::requests::Request::McpListResources) | `1` | `/mcp/list-resources/{channel}` | [`list_resources::response::Frame`](crate::shared::mcp::list_resources::response::Frame) |
//! | [`McpCallTool`](crate::container_proxy::requests::Request::McpCallTool) | `2` | `/mcp/call-tool/{channel}` | [`call_tool::response::Frame`](crate::shared::mcp::call_tool::response::Frame) |
//! | [`McpReadResource`](crate::container_proxy::requests::Request::McpReadResource) | `3` | `/mcp/read-resource/{channel}` | [`read_resource::response::Frame`](crate::shared::mcp::read_resource::response::Frame) |
//!
//! And, not an ask: `/mcp/notifications`, see [`notifications`].
//!
//! The four are what an MCP server is once the transport is taken
//! off it, less the stream: a client POSTs a JSON-RPC message for
//! each. The answer path carries exactly one message — the
//! exchange's own response type from
//! [`shared::mcp`](crate::shared::mcp), raw, result or error inside
//! it — and then the close. Nothing here defines a type of its own,
//! because a newtype over a type used raw would be ceremony.
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
//! side's statement, not an accident.

pub mod notifications;
