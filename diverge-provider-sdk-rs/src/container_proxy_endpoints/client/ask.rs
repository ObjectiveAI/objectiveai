//! One ask a proxy opens on a begin scope, owned.

use bytes::Bytes;

use super::super::{agents, tools};
use crate::shared::mcp;

/// What the proxy asks the server for on a begin scope, in the one
/// owned form both families' frames convert into, so the relay of
/// each is written once.
///
/// Owned, because the frame it came from borrows the message it
/// arrived in and an ask outlives that message: it is relayed to the
/// caller and answered whenever the caller answers.
#[derive(Debug, Clone)]
pub enum Ask {
    /// The proxy's half of a database connection, by the id the proxy
    /// minted.
    Postgres(u32),
    /// A command, as bytes in the CLI's own vocabulary.
    Command(Bytes),
    /// Read a vault key.
    VaultGet {
        /// The key.
        key: String,
    },
    /// Write a vault key.
    VaultSet {
        /// The key.
        key: String,
        /// The value, verbatim.
        value: Bytes,
    },
    /// Remove a vault key.
    VaultDelete {
        /// The key.
        key: String,
    },
    /// Hold a vault key's lock.
    VaultLock {
        /// The key.
        key: String,
        /// For how long, in seconds.
        ttl: u32,
    },
    /// Release a vault key's lock.
    VaultUnlock {
        /// The key.
        key: String,
    },
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

impl From<agents::begin::server::channel_request::Frame<'_>> for Ask {
    fn from(frame: agents::begin::server::channel_request::Frame<'_>) -> Self {
        use agents::begin::server::channel_request::Frame;
        match frame {
            Frame::Postgres(request) => Ask::Postgres(request.connection_id),
            Frame::Command(request) => Ask::Command(Bytes::copy_from_slice(request.0)),
            Frame::VaultGet(request) => Ask::VaultGet {
                key: request.key.to_owned(),
            },
            Frame::VaultSet(request) => Ask::VaultSet {
                key: request.key.to_owned(),
                value: Bytes::copy_from_slice(request.value),
            },
            Frame::VaultDelete(request) => Ask::VaultDelete {
                key: request.key.to_owned(),
            },
            Frame::VaultLock(request) => Ask::VaultLock {
                key: request.key.to_owned(),
                ttl: request.ttl,
            },
            Frame::VaultUnlock(request) => Ask::VaultUnlock {
                key: request.key.to_owned(),
            },
            Frame::McpListTools(request) => Ask::McpListTools(request),
            Frame::McpListResources(request) => Ask::McpListResources(request),
            Frame::McpCallTool(request) => Ask::McpCallTool(request),
            Frame::McpReadResource(request) => Ask::McpReadResource(request),
            Frame::McpNotifications(_) => Ask::McpNotifications,
        }
    }
}

impl From<tools::begin::server::channel_request::Frame<'_>> for Ask {
    fn from(frame: tools::begin::server::channel_request::Frame<'_>) -> Self {
        use tools::begin::server::channel_request::Frame;
        match frame {
            Frame::Postgres(request) => Ask::Postgres(request.connection_id),
            Frame::Command(request) => Ask::Command(Bytes::copy_from_slice(request.0)),
            Frame::VaultGet(request) => Ask::VaultGet {
                key: request.key.to_owned(),
            },
            Frame::VaultSet(request) => Ask::VaultSet {
                key: request.key.to_owned(),
                value: Bytes::copy_from_slice(request.value),
            },
            Frame::VaultDelete(request) => Ask::VaultDelete {
                key: request.key.to_owned(),
            },
            Frame::VaultLock(request) => Ask::VaultLock {
                key: request.key.to_owned(),
                ttl: request.ttl,
            },
            Frame::VaultUnlock(request) => Ask::VaultUnlock {
                key: request.key.to_owned(),
            },
            Frame::McpListTools(request) => Ask::McpListTools(request),
            Frame::McpListResources(request) => Ask::McpListResources(request),
            Frame::McpCallTool(request) => Ask::McpCallTool(request),
            Frame::McpReadResource(request) => Ask::McpReadResource(request),
            Frame::McpNotifications(_) => Ask::McpNotifications,
        }
    }
}
