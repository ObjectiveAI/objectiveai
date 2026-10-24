//! The proxy's own asks: what a container makes of the world
//! outside, in whichever family's frame the begin scope speaks.

use diverge_sdk::container_proxy::outside::endpoints::{agents, tools};
use diverge_sdk::shared::containers::{daemon, postgres};
use diverge_sdk::shared::mcp;

use crate::begin::Family;
use crate::encode::encoded;

/// One ask, before it is framed. The two begin families carry the
/// same asks in the same order, each in a frame of its own, less what
/// each family's container never makes — an agent container opens no
/// daemon connection, a tool container makes no tool call outward —
/// so this is the ask as the surface that makes it knows it, and
/// [`encoded`](Self::encoded) puts it in the frame the connection's
/// family reads, or answers that the family has no such ask.
pub enum Own {
    /// The proxy's half of a database connection the driver opened.
    Postgres(postgres::request::Postgres),
    /// Announce a daemon connection the program opened, by an id the
    /// proxy minted. A tool container's alone.
    Daemon(daemon::request::Daemon),
    /// What tools the caller's servers have. An agent container's
    /// alone, as the four after it are.
    McpListTools(mcp::list_tools::request::Request),
    /// What resources they have.
    McpListResources(mcp::list_resources::request::Request),
    /// Run one of their tools.
    McpCallTool(mcp::call_tool::request::Request),
    /// Read one of their resources.
    McpReadResource(mcp::read_resource::request::Request),
    /// Everything they say on their own account.
    McpNotifications(mcp::notifications::request::Request),
}

impl Own {
    /// The ask as the bytes of `family`'s channel request; `None` is
    /// an ask that would not encode, or one the family does not have
    /// — a daemon connection of an agent container's, a tool call
    /// outward of a tool container's.
    pub fn encoded(self, family: Family) -> Option<Vec<u8>> {
        match family {
            Family::Agents => encoded(&agents_frame(self)?),
            Family::Tools => encoded(&tools_frame(self)?),
        }
    }
}

/// The agents begin's frame for the ask, or `None` for the one ask
/// an agent container never makes: a daemon connection.
fn agents_frame(own: Own) -> Option<agents::begin::server::channel_request::Frame> {
    use agents::begin::server::channel_request::Frame;
    Some(match own {
        Own::Postgres(request) => Frame::Postgres(request),
        Own::Daemon(_) => return None,
        Own::McpListTools(request) => Frame::McpListTools(request),
        Own::McpListResources(request) => Frame::McpListResources(request),
        Own::McpCallTool(request) => Frame::McpCallTool(request),
        Own::McpReadResource(request) => Frame::McpReadResource(request),
        Own::McpNotifications(request) => Frame::McpNotifications(request),
    })
}

/// The tools begin's frame for the ask, or `None` for the five asks a
/// tool container never makes: the MCP exchanges outward.
fn tools_frame(own: Own) -> Option<tools::begin::server::channel_request::Frame> {
    use tools::begin::server::channel_request::Frame;
    Some(match own {
        Own::Postgres(request) => Frame::Postgres(request),
        Own::Daemon(request) => Frame::Daemon(request),
        Own::McpListTools(_)
        | Own::McpListResources(_)
        | Own::McpCallTool(_)
        | Own::McpReadResource(_)
        | Own::McpNotifications(_) => return None,
    })
}
