//! The tools family's own exchanges: the five MCP exchanges and the
//! close of a caller's MCP session, channels on the begin scope, each
//! with the session it names passed on to the proxy as it came.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::encoded::encoded;
use super::super::run::Run;
use diverge_sdk::provider::endpoints::containers::client::{ChannelStreamError, UnaryError};
use diverge_sdk::shared::mcp;
use diverge_sdk::shared::mcp::session::Sessioned;

/// What a tool container's caller opens, past the shared six.
#[derive(Debug)]
pub(crate) enum Exchange {
    /// What tools the server has.
    ListTools(Sessioned<mcp::list_tools::request::Request>),
    /// What resources it has.
    ListResources(Sessioned<mcp::list_resources::request::Request>),
    /// Run one tool.
    CallTool(Sessioned<mcp::call_tool::request::Request>),
    /// Read one resource.
    ReadResource(Sessioned<mcp::read_resource::request::Request>),
    /// Everything it says on its own account, on one caller's session.
    Notifications(Sessioned<mcp::notifications::request::Request>),
    /// One caller's session, closed.
    CloseSession(Sessioned<mcp::close_session::request::Request>),
}

/// Serve one, to the end: the proxy's answer on the caller's channel
/// — the result, or the MCP error, as the container's server gave it
/// — then the finish; or the finish alone where the proxy could not
/// serve it. Notifications are relayed until the proxy ends them or
/// the caller reads no more of the channel, which drops the proxy's
/// stream — telling the proxy the session's notifications are
/// abandoned — and finishes the channel. A close is relayed as the
/// four unary exchanges are: the proxy's `Closed` or its error, then
/// the finish.
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
        Exchange::CloseSession(request) => {
            let answer = match begin.close_session(&request.session).await {
                Ok(()) => encoded(&mcp::close_session::response::Frame::Closed),
                Err(UnaryError::Refused(error)) => encoded(&mcp::close_session::response::Frame::Error(error)),
                Err(_) => None,
            };
            run.respond(channel, answer).await;
        }
        Exchange::Notifications(request) => {
            if let Ok(mut notifications) = begin.notifications(request.session).await {
                loop {
                    let item = tokio::select! {
                        item = notifications.next() => item,
                        () = run.scope.answer_gone(channel) => break,
                    };
                    match item {
                        Some(Ok(notification)) => {
                            run.respond(channel, encoded(&mcp::notifications::response::Frame::Notification(notification)))
                                .await;
                        }
                        Some(Err(ChannelStreamError::Refused(error))) => {
                            run.respond(channel, encoded(&mcp::notifications::response::Frame::Error(error)))
                                .await;
                            break;
                        }
                        Some(Err(_)) | None => break,
                    }
                }
            }
        }
    }
    run.finish(channel).await;
}
