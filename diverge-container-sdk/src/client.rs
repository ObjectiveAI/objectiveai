//! The client: the proxy, from the inside.

use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::{ClientHandler, RoleClient, ServiceExt as _};

use super::Error;

/// The path the proxy serves the agent's MCP server on.
const MCP: &str = "/mcp/agent";

/// The proxy beside the container, as one client.
///
/// Dialed once and held for the program's life; every feature the
/// proxy carries is a method here, each feature in its own file —
/// the MCP exchanges in `mcp.rs`, the rest as they land.
///
/// `H` is the [`ClientHandler`] rmcp delivers MCP notifications to —
/// tools changed, resources changed, log messages. [`Client::connect`]
/// uses `()`, which drops them; [`Client::connect_with`] takes one.
pub struct Client<H: ClientHandler = ()> {
    /// The MCP session with the proxy's server at `/mcp/agent`.
    pub(crate) mcp: RunningService<RoleClient, H>,
}

impl Client<()> {
    /// Dial the proxy, with MCP notifications dropped.
    pub async fn connect() -> Result<Self, Error> {
        Self::connect_with(()).await
    }
}

impl<H: ClientHandler> Client<H> {
    /// Dial the proxy, with `handler` receiving the MCP notifications
    /// the caller's servers send.
    ///
    /// The one connection made here is the MCP session, initialized
    /// before this returns; the proxy's other features open their
    /// own connections as they are used.
    pub async fn connect_with(handler: H) -> Result<Self, Error> {
        let mcp = handler
            .serve(StreamableHttpClientTransport::from_uri(crate::url(MCP)))
            .await
            .map_err(Error::McpConnect)?;
        Ok(Self { mcp })
    }

    /// End every session with the proxy and wait for them to be gone.
    pub async fn close(self) -> Result<(), Error> {
        self.mcp.cancel().await.map(|_| ()).map_err(Error::McpClose)
    }
}
