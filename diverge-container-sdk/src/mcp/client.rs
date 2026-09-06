//! The client.

use rmcp::model::{
    CallToolRequestParams, CallToolResult, ListResourcesResult,
    ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResult,
};
use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::{ClientHandler, Peer, RoleClient, ServiceExt as _};

use super::Error;

/// The path the proxy serves the agent's MCP server on.
const PATH: &str = "/mcp/agent";

/// An MCP session with the proxy.
///
/// One session, dialed once and held for as long as the program
/// wants tools. Every method is one of the exchanges the proxy
/// relays to the caller, and the proxy's rules apply to each: a
/// call is asked again across a dead connection, and a tool that
/// failed comes back as a result rather than an error.
///
/// `H` is the [`ClientHandler`] rmcp delivers notifications to —
/// tools changed, resources changed, log messages. [`Client::connect`]
/// uses `()`, which drops them; [`Client::connect_with`] takes one.
pub struct Client<H: ClientHandler = ()> {
    service: RunningService<RoleClient, H>,
}

impl Client<()> {
    /// Dial the proxy and initialize, with notifications dropped.
    pub async fn connect() -> Result<Self, Error> {
        Self::connect_with(()).await
    }
}

impl<H: ClientHandler> Client<H> {
    /// Dial the proxy and initialize, with `handler` receiving the
    /// notifications the caller's servers send.
    pub async fn connect_with(handler: H) -> Result<Self, Error> {
        let service = handler
            .serve(StreamableHttpClientTransport::from_uri(crate::url(PATH)))
            .await
            .map_err(Error::Connect)?;
        Ok(Self { service })
    }

    /// What tools there are. [`None`] asks for the first page.
    pub async fn list_tools(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> Result<ListToolsResult, Error> {
        self.service
            .peer()
            .list_tools(params)
            .await
            .map_err(Error::Service)
    }

    /// What resources there are. [`None`] asks for the first page.
    pub async fn list_resources(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> Result<ListResourcesResult, Error> {
        self.service
            .peer()
            .list_resources(params)
            .await
            .map_err(Error::Service)
    }

    /// Run one tool.
    ///
    /// A tool that failed — the caller's server refused it, or the
    /// caller could not serve the call at all — comes back as a
    /// result with `is_error` set and the failure as its text; the
    /// proxy turns every such failure into one. An `Err` here is the
    /// MCP link itself failing, and nothing else.
    pub async fn call_tool(
        &self,
        params: CallToolRequestParams,
    ) -> Result<CallToolResult, Error> {
        self.service
            .peer()
            .call_tool(params)
            .await
            .map_err(Error::Service)
    }

    /// Read one resource, by URI.
    pub async fn read_resource(
        &self,
        params: ReadResourceRequestParams,
    ) -> Result<ReadResourceResult, Error> {
        self.service
            .peer()
            .read_resource(params)
            .await
            .map_err(Error::Service)
    }

    /// rmcp's own peer, for whatever the four methods do not cover.
    pub fn peer(&self) -> &Peer<RoleClient> {
        self.service.peer()
    }

    /// End the session and wait for it to be gone.
    pub async fn close(self) -> Result<(), Error> {
        self.service.cancel().await.map(|_| ()).map_err(Error::Close)
    }
}
