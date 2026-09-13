//! The tools family's own exchanges: the five MCP exchanges, channels
//! on the begin scope.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::encoded::encoded;
use super::super::run::Run;
use crate::endpoints::containers::client::{ChannelStreamError, UnaryError};
use crate::shared::mcp;

/// What a tool container's caller opens, past the shared five.
#[derive(Debug)]
pub(crate) enum Exchange {
    /// What tools the server has.
    ListTools(mcp::list_tools::request::Request),
    /// What resources it has.
    ListResources(mcp::list_resources::request::Request),
    /// Run one tool.
    CallTool(mcp::call_tool::request::Request),
    /// Read one resource.
    ReadResource(mcp::read_resource::request::Request),
    /// Everything it says on its own account.
    Notifications,
}

/// Serve one, to the end: the proxy's answer on the caller's channel
/// — the result, or the MCP error, as the container's server gave it
/// — then the finish; or the finish alone where the proxy could not
/// serve it.
pub(crate) async fn serve(run: Arc<Run>, channel: u32, exchange: Exchange) {
    let Some(begin) = run.begin.tools() else {
        // An agent container has no MCP server of its own; nothing
        // classifies into this on one.
        run.finish(channel).await;
        return;
    };
    match exchange {
        Exchange::ListTools(request) => {
            let answer = match begin.list_tools(request).await {
                Ok(result) => encoded(&mcp::list_tools::response::Frame::Result(result)),
                Err(UnaryError::Refused(error)) => encoded(&mcp::list_tools::response::Frame::Error(error)),
                Err(_) => None,
            };
            run.respond(channel, answer).await;
        }
        Exchange::ListResources(request) => {
            let answer = match begin.list_resources(request).await {
                Ok(result) => encoded(&mcp::list_resources::response::Frame::Result(result)),
                Err(UnaryError::Refused(error)) => encoded(&mcp::list_resources::response::Frame::Error(error)),
                Err(_) => None,
            };
            run.respond(channel, answer).await;
        }
        Exchange::CallTool(request) => {
            let answer = match begin.call_tool(request).await {
                Ok(result) => encoded(&mcp::call_tool::response::Frame::Result(result)),
                Err(UnaryError::Refused(error)) => encoded(&mcp::call_tool::response::Frame::Error(error)),
                Err(_) => None,
            };
            run.respond(channel, answer).await;
        }
        Exchange::ReadResource(request) => {
            let answer = match begin.read_resource(request).await {
                Ok(result) => encoded(&mcp::read_resource::response::Frame::Result(result)),
                Err(UnaryError::Refused(error)) => encoded(&mcp::read_resource::response::Frame::Error(error)),
                Err(_) => None,
            };
            run.respond(channel, answer).await;
        }
        Exchange::Notifications => {
            if let Ok(mut notifications) = begin.notifications().await {
                while let Some(item) = notifications.next().await {
                    match item {
                        Ok(notification) => {
                            run.respond(channel, encoded(&mcp::notifications::response::Frame::Notification(notification)))
                                .await;
                        }
                        Err(ChannelStreamError::Refused(error)) => {
                            run.respond(channel, encoded(&mcp::notifications::response::Frame::Error(error)))
                                .await;
                            break;
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    }
    run.finish(channel).await;
}
