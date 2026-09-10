//! The tools family's own exchanges: the caller into the container's
//! MCP server.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::encoded::encoded;
use super::super::run::Run;
use crate::container_proxy::tool;
use crate::shared::mcp;

/// What a tool container's caller — its runner, or a connector —
/// opens, past the shared five: the five MCP exchanges, carried to
/// the container's own server through the proxy's `/tool/*`.
#[derive(Debug)]
pub(crate) enum Exchange {
    ListTools(mcp::list_tools::request::Request),
    ListResources(mcp::list_resources::request::Request),
    CallTool(mcp::call_tool::request::Request),
    ReadResource(mcp::read_resource::request::Request),
    Notifications,
}

/// Serve one, to the end.
pub(crate) async fn serve(run: Arc<Run>, channel: u32, exchange: Exchange) {
    match exchange {
        Exchange::ListTools(request) => {
            let answer = tool::list_tools::execute::execute(&run.client, &request).await.ok();
            unary(run, channel, answer.as_ref()).await
        }
        Exchange::ListResources(request) => {
            let answer = tool::list_resources::execute::execute(&run.client, &request).await.ok();
            unary(run, channel, answer.as_ref()).await
        }
        Exchange::CallTool(request) => {
            let answer = tool::call_tool::execute::execute(&run.client, &request).await.ok();
            unary(run, channel, answer.as_ref()).await
        }
        Exchange::ReadResource(request) => {
            let answer = tool::read_resource::execute::execute(&run.client, &request).await.ok();
            unary(run, channel, answer.as_ref()).await
        }
        Exchange::Notifications => notifications(run, channel).await,
    }
}

/// The one frame the container's server answered — its result or its
/// own error, both the caller's — then the finish; a proxy that could
/// not carry the exchange is the finish alone.
async fn unary<T: crate::encode::Encode>(run: Arc<Run>, channel: u32, answer: Option<&T>) {
    run.respond(channel, answer.and_then(encoded)).await;
    run.finish(channel).await;
}

/// The server's notifications as they come, then the finish; the
/// proxy's `Error` last, if the server could not be reached.
async fn notifications(run: Arc<Run>, channel: u32) {
    if let Ok(mut notifications) = tool::notifications::execute::execute(&run.client).await {
        while let Some(item) = notifications.next().await {
            match item {
                Ok(notification) => {
                    run.respond(channel, encoded(&mcp::notifications::response::Frame::Notification(notification)))
                        .await
                }
                Err(tool::notifications::execute::ExecuteStreamError::Refused(error)) => {
                    run.respond(channel, encoded(&mcp::notifications::response::Frame::Error(error))).await;
                    break;
                }
                Err(_) => break,
            }
        }
    }
    run.finish(channel).await;
}
