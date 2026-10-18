//! One ask a proxy opens on a begin scope, owned.

use super::super::endpoints::{agents, tools};
use crate::shared::mcp;

/// What the proxy asks the server for on a begin scope, in the one
/// owned form both families' frames convert into, so the relay of
/// each is written once: an agent container's are the database
/// connections and the five MCP exchanges outward, a tool container's
/// the database connections and the daemon connection.
///
/// Owned, because the frame it came from borrows the message it
/// arrived in and an ask outlives that message: it is relayed to the
/// caller and answered whenever the caller answers.
#[derive(Debug, Clone)]
pub enum Ask {
    /// The proxy's half of a database connection, by the id the proxy
    /// minted.
    Postgres(u32),
    /// A daemon connection the program opened, by the id the proxy
    /// minted.
    Daemon(u32),
    /// What tools the caller's servers have.
    McpListTools(mcp::list_tools::request::Request),
    /// What resources they have.
    McpListResources(mcp::list_resources::request::Request),
    /// Run one of their tools.
    McpCallTool(mcp::call_tool::request::Request),
    /// Read one of their resources.
    McpReadResource(mcp::read_resource::request::Request),
    /// Everything they say on their own account.
    McpNotifications,
}

impl From<agents::begin::server::channel_request::Frame> for Ask {
    fn from(frame: agents::begin::server::channel_request::Frame) -> Self {
        use agents::begin::server::channel_request::Frame;
        match frame {
            Frame::Postgres(request) => Ask::Postgres(request.connection_id),
            Frame::McpListTools(request) => Ask::McpListTools(request),
            Frame::McpListResources(request) => Ask::McpListResources(request),
            Frame::McpCallTool(request) => Ask::McpCallTool(request),
            Frame::McpReadResource(request) => Ask::McpReadResource(request),
            Frame::McpNotifications(_) => Ask::McpNotifications,
        }
    }
}

impl From<tools::begin::server::channel_request::Frame> for Ask {
    fn from(frame: tools::begin::server::channel_request::Frame) -> Self {
        use tools::begin::server::channel_request::Frame;
        match frame {
            Frame::Postgres(request) => Ask::Postgres(request.connection_id),
            Frame::Daemon(request) => Ask::Daemon(request.connection_id),
        }
    }
}
