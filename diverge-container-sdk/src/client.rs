//! The client: the proxy, from the inside.

use rmcp::model::{ClientCapabilities, ClientInfo, Implementation};
use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::{RoleClient, ServiceExt as _};
use tokio::sync::OnceCell;

use super::Error;

/// The path the proxy serves the agent's MCP server on.
const MCP: &str = "/mcp/agent";

/// The proxy beside the container, as one client.
///
/// Made without a connection and held for the program's life; every
/// feature the proxy carries is a method here, each feature in its
/// own file — the MCP exchanges in `mcp.rs`, the vault in `vault.rs`,
/// the rest as they land. MCP opens its session the first time it is
/// used; the vault and the features after it speak plain HTTP to the
/// proxy, one request per call, nothing kept between. There is
/// nothing to connect and nothing to close: a session ends when the
/// client is dropped.
#[derive(Default)]
pub struct Client {
    /// The MCP session with the proxy's server at `/mcp/agent`, made
    /// by the first MCP method that needs it.
    mcp: OnceCell<RunningService<RoleClient, ClientInfo>>,
    /// The HTTP client the plain-HTTP features share. A handle, not
    /// a connection: making it is no I/O.
    http: reqwest::Client,
}

impl Client {
    /// The client. No I/O happens here.
    pub fn new() -> Self {
        Self::default()
    }

    /// The HTTP client, for the features that are one request per
    /// call.
    pub(crate) fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// The MCP session, dialed and initialized on first use.
    ///
    /// Concurrent first callers wait for the one dial; a dial that
    /// failed leaves nothing behind, so the next call tries again.
    /// The session's handler is fixed: it answers the server's
    /// requests with refusals and ignores its notifications, exactly
    /// as rmcp's default does, and announces this crate as the
    /// client rather than rmcp.
    pub(crate) async fn mcp(
        &self,
    ) -> Result<&RunningService<RoleClient, ClientInfo>, Error> {
        self.mcp
            .get_or_try_init(|| async {
                ClientInfo::new(
                    ClientCapabilities::default(),
                    Implementation::new("diverge", env!("CARGO_PKG_VERSION")),
                )
                .serve(StreamableHttpClientTransport::from_uri(crate::url(MCP)))
                .await
                .map_err(Error::McpConnect)
            })
            .await
    }
}
