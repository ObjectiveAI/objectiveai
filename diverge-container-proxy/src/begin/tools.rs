//! A tool container's begin: the arguments registered, the schema and
//! the exchanges with the tool's own MCP server served.

use std::sync::Arc;

use diverge_sdk::container_proxy::outside::endpoints::tools::begin::client::{channel_request, request};
use diverge_sdk::container_proxy::outside::endpoints::tools::begin::server::response;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::error::Error;

use super::Family;
use crate::encode::encoded;
use crate::proxy::{Begun, Proxy};
use crate::stamp::Stamp;
use crate::{inside, program, tool};

/// Serve the begin for the connection's life.
///
/// A second begin on the connection is `Error`, then the finish, and
/// the first goes on. Otherwise the arguments are registered with the
/// program's server — its refusal is the `Error`, in its own words,
/// and the connection has not begun — and then `Begun` goes out
/// carrying the tools the program answered with, the
/// scope is published to every surface inside the container, and
/// every channel the server opens on the scope is served on a task
/// of its own: the schema once, one exchange with the tool's server
/// each — its answer carrying the container's image under `_meta` —
/// the notifications for as long as the channel lives, the server's
/// half of a database connection until the driver hangs up.
/// A channel this end cannot read is finished with nothing.
pub async fn tools(proxy: Arc<Proxy>, scope: ScopeHandle, frame: request::Frame) {
    if !proxy.claim_begin().await {
        refuse(&scope, super::agents::begun()).await;
        return;
    }
    let stamp = Stamp::new(&frame.image);
    let tools = match program::register(&proxy.upstream, frame.arguments).await {
        Ok(tools) => tools,
        Err(error) => {
            proxy.release_begin().await;
            refuse(&scope, error).await;
            return;
        }
    };
    let scope = Arc::new(scope);
    if let Some(payload) = encoded(&response::Frame::Begun(tools)) {
        scope.send_response(&payload).await;
    }
    proxy.publish(Begun {
        scope: Arc::clone(&scope),
        family: Family::Tools,
        stamp: stamp.clone(),
    });

    while let Some(bytes) = scope.recv_channel_request().await {
        let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
            continue;
        };
        let proxy = Arc::clone(&proxy);
        let scope = Arc::clone(&scope);
        let stamp = stamp.clone();
        match channel_request::Frame::decode(payload) {
            Ok(channel_request::Frame::Postgres(request)) => {
                tokio::spawn(inside::postgres::attach(proxy, scope, channel, request.connection_id));
            }
            Ok(channel_request::Frame::Schema) => {
                tokio::spawn(program::schema(proxy, scope, channel));
            }
            Ok(channel_request::Frame::McpListTools(request)) => {
                tokio::spawn(async move { tool::list_tools(&proxy.tool, &stamp, &scope, channel, request).await });
            }
            Ok(channel_request::Frame::McpListResources(request)) => {
                tokio::spawn(async move { tool::list_resources(&proxy.tool, &stamp, &scope, channel, request).await });
            }
            Ok(channel_request::Frame::McpCallTool(request)) => {
                tokio::spawn(async move { tool::call_tool(&proxy.tool, &stamp, &scope, channel, request).await });
            }
            Ok(channel_request::Frame::McpReadResource(request)) => {
                tokio::spawn(async move { tool::read_resource(&proxy.tool, &stamp, &scope, channel, request).await });
            }
            Ok(channel_request::Frame::McpNotifications(_)) => {
                tokio::spawn(async move { tool::notifications(&proxy.tool, &stamp, &scope, channel).await });
            }
            Err(_) => {
                tokio::spawn(async move { scope.send_channel_response_finish(channel).await });
            }
        }
    }
}

/// The `Error`, then the finish: the connection has not begun.
async fn refuse(scope: &ScopeHandle, error: Error) {
    if let Some(payload) = encoded(&response::Frame::Error(error)) {
        scope.send_response(&payload).await;
    }
    scope.send_response_finish().await;
}
