//! MCP: the four exchanges the proxy relays to the caller's servers.
//!
//! At `/mcp` the proxy is a compliant MCP server whose answers
//! all live with the caller: what it lists and what it calls are the
//! caller's servers, relayed. The first of these methods to be called
//! dials the session; the rest share it. The proxy's rules apply to
//! each exchange — an ask is asked again across a dead connection,
//! and a tool that failed comes back as a result rather than an
//! error.

use rmcp::model::{
    CallToolRequestParams, CallToolResult, ListResourcesResult,
    ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResult,
};
use rmcp::{Peer, RoleClient};

use crate::{Client, Error};

/// The path the proxy serves the program's MCP server on.
const MCP: &str = "/mcp";

/// The URL of the proxy's MCP server, for an MCP client that is not
/// this crate's.
///
/// A program whose MCP client is somebody else's — a subprocess it
/// launches, say, handed a server list — points it here. The one
/// place the address is spelled: built from the provider SDK's port
/// and the path the proxy serves, so a program that uses this never
/// names either, and a port that moves moves for everyone at once.
/// [`Client`]'s own session dials the same URL.
pub fn mcp_url() -> String {
    crate::url(MCP)
}

impl Client {
    /// What tools there are. [`None`] asks for the first page.
    pub async fn mcp_list_tools(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> Result<ListToolsResult, Error> {
        self.mcp()
            .await?
            .peer()
            .list_tools(params)
            .await
            .map_err(Error::McpService)
    }

    /// What resources there are. [`None`] asks for the first page.
    pub async fn mcp_list_resources(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> Result<ListResourcesResult, Error> {
        self.mcp()
            .await?
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
    pub async fn mcp_call_tool(
        &self,
        params: CallToolRequestParams,
    ) -> Result<CallToolResult, Error> {
        self.mcp()
            .await?
            .peer()
            .call_tool(params)
            .await
            .map_err(Error::McpService)
    }

    /// Read one resource, by URI.
    pub async fn mcp_read_resource(
        &self,
        params: ReadResourceRequestParams,
    ) -> Result<ReadResourceResult, Error> {
        self.mcp()
            .await?
            .peer()
            .read_resource(params)
            .await
            .map_err(Error::McpService)
    }

    /// rmcp's own peer for the MCP session, for whatever the four
    /// methods do not cover. Dials the session if none exists yet.
    pub async fn mcp_peer(&self) -> Result<&Peer<RoleClient>, Error> {
        Ok(self.mcp().await?.peer())
    }
}
