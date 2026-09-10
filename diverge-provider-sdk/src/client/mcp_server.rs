//! The caller's MCP servers, as one server the container calls.

use std::future::Future;
use std::pin::Pin;

use futures_util::Stream;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ErrorData, ListResourcesResult, ListToolsResult,
    PaginatedRequestParams, ReadResourceRequestParams, ReadResourceResult, ServerNotification,
};

/// The five MCP exchanges a container makes outward, answered by the
/// caller — the agent's tool calls go to servers that live with the
/// caller, and this is how they are reached.
///
/// One logical server: however many the caller aggregates behind it,
/// the container sees one tool list and one resource list. Four
/// exchanges are answered once; notifications is a stream the caller
/// pushes for as long as the container listens. An [`ErrorData`] is
/// an MCP error relayed as MCP says it — `-32601` is "no such tool",
/// `-32602` "the arguments were wrong" — and is not this crate's
/// [`Error`](crate::shared::error::Error).
pub trait McpServer: Send + Sync {
    /// `tools/list`.
    fn list_tools(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> + Send;

    /// `resources/list`.
    fn list_resources(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> impl Future<Output = Result<ListResourcesResult, ErrorData>> + Send;

    /// `tools/call`.
    fn call_tool(
        &self,
        params: CallToolRequestParams,
    ) -> impl Future<Output = Result<CallToolResult, ErrorData>> + Send;

    /// `resources/read`.
    fn read_resource(
        &self,
        params: ReadResourceRequestParams,
    ) -> impl Future<Output = Result<ReadResourceResult, ErrorData>> + Send;

    /// Everything the servers say on their own account, for as long
    /// as the stream lives; an `Err` is the last thing it says.
    fn notifications(
        &self,
    ) -> impl Future<
        Output = Pin<Box<dyn Stream<Item = Result<ServerNotification, ErrorData>> + Send + 'static>>,
    > + Send;
}
