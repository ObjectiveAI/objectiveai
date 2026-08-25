//! The MCP server inside an agentic_loop container.
//!
//! To the agent beside it, a fully compliant MCP server on the
//! container's loopback. To the provider outside, a WebSocket
//! listener: the provider connects in, and every MCP exchange the
//! agent asks for is carried out over that socket — MCP over
//! WebSocket — to be answered by the caller's servers on the far side
//! of the provider protocol.

fn main() {}
