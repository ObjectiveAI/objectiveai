//! The proxy's own asks: the twelve a container makes of the world
//! outside, in whichever family's frame the begin scope speaks.

use diverge_provider_sdk::container_proxy_endpoints::{agents, tools};
use diverge_provider_sdk::shared::containers::{command, postgres, vault};
use diverge_provider_sdk::shared::mcp;

use crate::begin::Family;
use crate::encode::encoded;

/// One ask, before it is framed. The two begin families carry the
/// same twelve in the same order, each in a frame of its own; this
/// is the ask as the surface that makes it knows it, and
/// [`encoded`](Self::encoded) puts it in the frame the connection's
/// family reads.
pub enum Own<'a> {
    /// The proxy's half of a database connection the driver opened.
    Postgres(postgres::request::Postgres),
    /// Run a command the container asked for.
    Command(command::request::Request<'a>),
    /// Read a vault key.
    VaultGet(vault::get::request::Request<'a>),
    /// Write a vault key.
    VaultSet(vault::set::request::Request<'a>),
    /// Remove a vault key.
    VaultDelete(vault::delete::request::Request<'a>),
    /// Hold a vault key's lock.
    VaultLock(vault::lock::request::Request<'a>),
    /// Release a vault key's lock.
    VaultUnlock(vault::unlock::request::Request<'a>),
    /// What tools the caller's servers have.
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

impl<'a> Own<'a> {
    /// The ask as the bytes of `family`'s channel request; `None` is
    /// an ask that would not encode.
    pub fn encoded(self, family: Family) -> Option<Vec<u8>> {
        match family {
            Family::Agents => encoded(&agents::begin::server::channel_request::Frame::from(self)),
            Family::Tools => encoded(&tools::begin::server::channel_request::Frame::from(self)),
        }
    }
}

impl<'a> From<Own<'a>> for agents::begin::server::channel_request::Frame<'a> {
    fn from(own: Own<'a>) -> Self {
        match own {
            Own::Postgres(request) => Self::Postgres(request),
            Own::Command(request) => Self::Command(request),
            Own::VaultGet(request) => Self::VaultGet(request),
            Own::VaultSet(request) => Self::VaultSet(request),
            Own::VaultDelete(request) => Self::VaultDelete(request),
            Own::VaultLock(request) => Self::VaultLock(request),
            Own::VaultUnlock(request) => Self::VaultUnlock(request),
            Own::McpListTools(request) => Self::McpListTools(request),
            Own::McpListResources(request) => Self::McpListResources(request),
            Own::McpCallTool(request) => Self::McpCallTool(request),
            Own::McpReadResource(request) => Self::McpReadResource(request),
            Own::McpNotifications(request) => Self::McpNotifications(request),
        }
    }
}

impl<'a> From<Own<'a>> for tools::begin::server::channel_request::Frame<'a> {
    fn from(own: Own<'a>) -> Self {
        match own {
            Own::Postgres(request) => Self::Postgres(request),
            Own::Command(request) => Self::Command(request),
            Own::VaultGet(request) => Self::VaultGet(request),
            Own::VaultSet(request) => Self::VaultSet(request),
            Own::VaultDelete(request) => Self::VaultDelete(request),
            Own::VaultLock(request) => Self::VaultLock(request),
            Own::VaultUnlock(request) => Self::VaultUnlock(request),
            Own::McpListTools(request) => Self::McpListTools(request),
            Own::McpListResources(request) => Self::McpListResources(request),
            Own::McpCallTool(request) => Self::McpCallTool(request),
            Own::McpReadResource(request) => Self::McpReadResource(request),
            Own::McpNotifications(request) => Self::McpNotifications(request),
        }
    }
}
