//! A tool container's begin: `Begun` at once, the exchanges with the
//! tool's own MCP server served.

use std::sync::Arc;

use diverge_provider_sdk::container_proxy_endpoints::tools::begin::client::channel_request;
use diverge_provider_sdk::container_proxy_endpoints::tools::begin::server::response;
use diverge_provider_sdk::decode::Decode as _;
use diverge_provider_sdk::frame::client::ClientFrame;
use diverge_provider_sdk::server::scope_handle::ScopeHandle;

use super::Family;
use crate::encode::encoded;
use crate::proxy::{Begun, Proxy};
use crate::{inside, tool};

/// Serve the begin for the connection's life.
///
/// A second begin on the connection is `Error`, then the finish, and
/// the first goes on. Otherwise `Begun` goes out at once — there is
/// nothing to hand a tool container — the scope is published to
/// every surface inside the container, and every channel the server
/// opens on the scope is served on a task of its own: one exchange
/// with the tool's server each, the notifications for as long as the
/// channel lives, the server's half of a database connection until
/// the driver hangs up. A channel this end cannot read is finished
/// with nothing.
pub async fn tools(proxy: Arc<Proxy>, scope: ScopeHandle) {
    if !proxy.claim_begin().await {
        if let Some(payload) = encoded(&response::Frame::Error(super::agents::begun())) {
            scope.send_response(&payload).await;
        }
        scope.send_response_finish().await;
        return;
    }
    let scope = Arc::new(scope);
    if let Some(payload) = encoded(&response::Frame::Begun) {
        scope.send_response(&payload).await;
    }
    proxy.publish(Begun {
        scope: Arc::clone(&scope),
        family: Family::Tools,
    });

    while let Some(bytes) = scope.recv_channel_request().await {
        let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
            continue;
        };
        let proxy = Arc::clone(&proxy);
        let scope = Arc::clone(&scope);
        match channel_request::Frame::decode(payload) {
            Ok(channel_request::Frame::Postgres(request)) => {
                tokio::spawn(inside::postgres::attach(proxy, scope, channel, request.connection_id));
            }
            Ok(channel_request::Frame::McpListTools(request)) => {
                tokio::spawn(async move { tool::list_tools(&proxy.tool, &scope, channel, request).await });
            }
            Ok(channel_request::Frame::McpListResources(request)) => {
                tokio::spawn(async move { tool::list_resources(&proxy.tool, &scope, channel, request).await });
            }
            Ok(channel_request::Frame::McpCallTool(request)) => {
                tokio::spawn(async move { tool::call_tool(&proxy.tool, &scope, channel, request).await });
            }
            Ok(channel_request::Frame::McpReadResource(request)) => {
                tokio::spawn(async move { tool::read_resource(&proxy.tool, &scope, channel, request).await });
            }
            Ok(channel_request::Frame::McpNotifications(_)) => {
                tokio::spawn(async move { tool::notifications(&proxy.tool, &scope, channel).await });
            }
            Err(_) => {
                tokio::spawn(async move { scope.send_channel_response_finish(channel).await });
            }
        }
    }
}
