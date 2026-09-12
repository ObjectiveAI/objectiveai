//! The container's tool calls outward, to the caller's MCP servers.

use std::sync::Arc;

use futures_util::StreamExt as _;
use rmcp::model::{CallToolRequestParams, PaginatedRequestParams, ReadResourceRequestParams};

use super::send::{Stop, finish, respond};
use crate::client::McpServer;
use crate::client::handle::Handle;
use crate::shared::mcp;

/// One frame — the result, or the server's error — then the finish.
pub(crate) async fn list_tools<M: McpServer>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    params: Option<PaginatedRequestParams>,
    server: Arc<M>,
) -> Result<(), Stop> {
    let frame = match server.list_tools(params).await {
        Ok(result) => mcp::list_tools::response::Frame::Result(result),
        Err(error) => mcp::list_tools::response::Frame::Error(error),
    };
    respond(handle, scope, channel, &frame).await?;
    finish(handle, scope, channel).await
}

/// One frame — the result, or the server's error — then the finish.
pub(crate) async fn list_resources<M: McpServer>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    params: Option<PaginatedRequestParams>,
    server: Arc<M>,
) -> Result<(), Stop> {
    let frame = match server.list_resources(params).await {
        Ok(result) => mcp::list_resources::response::Frame::Result(result),
        Err(error) => mcp::list_resources::response::Frame::Error(error),
    };
    respond(handle, scope, channel, &frame).await?;
    finish(handle, scope, channel).await
}

/// One frame — the result, or the server's error — then the finish.
pub(crate) async fn call_tool<M: McpServer>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    params: CallToolRequestParams,
    server: Arc<M>,
) -> Result<(), Stop> {
    let frame = match server.call_tool(params).await {
        Ok(result) => mcp::call_tool::response::Frame::Result(result),
        Err(error) => mcp::call_tool::response::Frame::Error(error),
    };
    respond(handle, scope, channel, &frame).await?;
    finish(handle, scope, channel).await
}

/// One frame — the result, or the server's error — then the finish.
pub(crate) async fn read_resource<M: McpServer>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    params: ReadResourceRequestParams,
    server: Arc<M>,
) -> Result<(), Stop> {
    let frame = match server.read_resource(params).await {
        Ok(result) => mcp::read_resource::response::Frame::Result(result),
        Err(error) => mcp::read_resource::response::Frame::Error(error),
    };
    respond(handle, scope, channel, &frame).await?;
    finish(handle, scope, channel).await
}

/// One frame per notification for as long as the servers speak; an
/// error is the last frame; then the finish.
pub(crate) async fn notifications<M: McpServer>(
    handle: &Handle,
    scope: u32,
    channel: u32,
    server: Arc<M>,
) -> Result<(), Stop> {
    let notifications = server.notifications().await;
    let mut notifications = std::pin::pin!(notifications);
    while let Some(notification) = notifications.next().await {
        match notification {
            Ok(notification) => {
                let frame = mcp::notifications::response::Frame::Notification(notification);
                respond(handle, scope, channel, &frame).await?;
            }
            Err(error) => {
                respond(handle, scope, channel, &mcp::notifications::response::Frame::Error(error)).await?;
                break;
            }
        }
    }
    finish(handle, scope, channel).await
}
