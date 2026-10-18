//! A tool container's begin: the arguments registered, the image's
//! `/DIVERGE.md` and the exchanges with the tool's own MCP server
//! served.

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
use crate::{info, inside, program, tool};

/// Serve the begin for the connection's life.
///
/// A second begin on the connection is `Error`, then the finish, and
/// the first goes on. Otherwise the arguments and the template are
/// registered with the program's server — its refusal is the `Error`,
/// in its own words, and the connection has not begun — and then
/// `Begun` goes out carrying the secrets the program takes, by name
/// and mode, the dependencies it answered with dropped, since a tool
/// container declares none; the declaration is kept first, and the
/// activity's gate set from it — open for a program that takes no
/// secret, closed until the server's first feed for one that does.
/// The scope is published to every surface inside the container, and
/// every channel the server opens on the scope is served on a task
/// of its own: the image's `/DIVERGE.md` read from the root at every
/// ask, the program asked nothing; one exchange with the tool's
/// server each — its answer as the tool's server gave it, the exchange
/// entered and left as the container's activity — the notifications
/// for as long as the channel lives, a feed of the secrets answered
/// once the program has applied them, the server's half of a
/// database connection until the driver hangs up. A channel this end
/// cannot read is finished with nothing. The loop is raced against
/// the proxy ending the run — a collect that failed, its `Error` on
/// the main stream already — and finishes the scope then; this loop
/// alone may, since reading the inbox holds its lock. Otherwise the
/// scope is the connection's life.
pub async fn tools(proxy: Arc<Proxy>, scope: ScopeHandle, frame: request::Frame) {
    if !proxy.claim_begin().await {
        refuse(&scope, super::agents::begun()).await;
        return;
    }
    let registered = match program::register(&proxy.upstream, frame.arguments, frame.template).await {
        Ok(registered) => registered,
        Err(error) => {
            proxy.release_begin().await;
            refuse(&scope, error).await;
            return;
        }
    };
    let scope = Arc::new(scope);
    proxy.set_declaration(registered.secrets.clone());
    proxy.activity.set_open(registered.secrets.is_empty());
    let begun = response::Registered {
        secrets: registered.secrets,
    };
    if let Some(payload) = encoded(&response::Frame::Begun(begun)) {
        scope.send_response(&payload).await;
    }
    proxy.publish(Begun {
        scope: Arc::clone(&scope),
        family: Family::Tools,
    });

    loop {
        let bytes = tokio::select! {
            bytes = scope.recv_channel_request() => match bytes {
                Some(bytes) => bytes,
                None => break,
            },
            _ = proxy.ended() => break,
        };
        let Ok(ClientFrame::ChannelRequest { channel, payload, .. }) = ClientFrame::decode(&bytes) else {
            continue;
        };
        let proxy = Arc::clone(&proxy);
        let scope = Arc::clone(&scope);
        match channel_request::Frame::decode(payload) {
            Ok(channel_request::Frame::Daemon(request)) => {
                tokio::spawn(inside::daemon::attach(proxy, scope, channel, request.connection_id));
            }
            Ok(channel_request::Frame::Postgres(request)) => {
                tokio::spawn(inside::postgres::attach(proxy, scope, channel, request.connection_id));
            }
            Ok(channel_request::Frame::Info) => {
                tokio::spawn(info::info(scope, channel));
            }
            Ok(channel_request::Frame::McpListTools(request)) => {
                tokio::spawn(async move { tool::list_tools(&proxy, &scope, channel, request).await });
            }
            Ok(channel_request::Frame::McpListResources(request)) => {
                tokio::spawn(async move { tool::list_resources(&proxy, &scope, channel, request).await });
            }
            Ok(channel_request::Frame::McpCallTool(request)) => {
                tokio::spawn(async move { tool::call_tool(&proxy, &scope, channel, request).await });
            }
            Ok(channel_request::Frame::McpReadResource(request)) => {
                tokio::spawn(async move { tool::read_resource(&proxy, &scope, channel, request).await });
            }
            Ok(channel_request::Frame::McpNotifications(_)) => {
                tokio::spawn(async move { tool::notifications(&proxy.tool, &scope, channel).await });
            }
            Ok(channel_request::Frame::Secrets(values)) => {
                tokio::spawn(tool::feed(proxy, scope, channel, values));
            }
            Err(_) => {
                tokio::spawn(async move { scope.send_channel_response_finish(channel).await });
            }
        }
    }
    scope.send_response_finish().await;
}

/// The `Error`, then the finish: the connection has not begun.
async fn refuse(scope: &ScopeHandle, error: Error) {
    if let Some(payload) = encoded(&response::Frame::Error(error)) {
        scope.send_response(&payload).await;
    }
    scope.send_response_finish().await;
}
