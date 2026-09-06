//! MCP: the four exchanges the proxy relays to the caller's servers.
//!
//! At `/mcp/agent` the proxy is a compliant MCP server whose answers
//! all live with the caller: what it lists and what it calls are the
//! caller's servers, relayed. The proxy's rules apply to each
//! exchange — an ask is asked again across a dead connection, and a
//! tool that failed comes back as a result rather than an error.

use rmcp::model::{
    CallToolRequestParams, CallToolResult, ListResourcesResult,
    ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResult,
};
use rmcp::{ClientHandler, Peer, RoleClient};

use crate::{Client, Error};

impl<H: ClientHandler> Client<H> {
    /// What tools there are. [`None`] asks for the first page.
    pub async fn list_tools(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> Result<ListToolsResult, Error> {
        self.mcp
            .peer()
            .list_tools(params)
            .await
            .map_err(Error::McpService)
    }

    /// What resources there are. [`None`] asks for the first page.
    pub async fn list_resources(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> Result<ListResourcesResult, Error> {
        self.mcp
            .peer()
            .list_resources(params)
            .await
            .map_err(Error::McpService)
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
        self.mcp
            .peer()
            .call_tool(params)
            .await
            .map_err(Error::McpService)
    }

    /// Read one resource, by URI.
    pub async fn read_resource(
        &self,
        params: ReadResourceRequestParams,
    ) -> Result<ReadResourceResult, Error> {
        self.mcp
            .peer()
            .read_resource(params)
            .await
            .map_err(Error::McpService)
    }

    /// rmcp's own peer for the MCP session, for whatever the four
    /// methods do not cover.
    pub fn mcp_peer(&self) -> &Peer<RoleClient> {
        self.mcp.peer()
    }
}
