//! An agent container's begin: the arguments registered, the queue's
//! driver started, the family's channels served.

use std::sync::Arc;

use diverge_provider_sdk::container_proxy_endpoints::agents::begin::client::{channel_request, request};
use diverge_provider_sdk::container_proxy_endpoints::agents::begin::server::response;
use diverge_provider_sdk::decode::Decode as _;
use diverge_provider_sdk::frame::client::ClientFrame;
use diverge_provider_sdk::server::scope_handle::ScopeHandle;
use diverge_provider_sdk::shared::error::Error;

use super::Family;
use crate::encode::encoded;
use crate::proxy::{Begun, Proxy};
use crate::stamp::Stamp;
use crate::{agent, inside, program};

/// Serve the begin for the connection's life.
///
/// A second begin on the connection is `Error`, then the finish, and
/// the first goes on. Otherwise the arguments are registered with the
/// program's server — its refusal is the `Error`, in its own words,
/// and the connection has not begun — and then `Begun` goes out
/// carrying the tools the program answered with, the
/// scope is published to every surface inside the container, the
/// queue's driver and the resident notifications ask are started,
/// and every channel the server opens on the scope is served on a
/// task of its own: a channel this end cannot read is finished with
/// nothing. The scope is never finished by this end: it is the
/// connection's life.
pub async fn agents(proxy: Arc<Proxy>, scope: ScopeHandle, frame: request::Frame) {
    if !proxy.claim_begin().await {
        refuse(&scope, begun()).await;
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
        family: Family::Agents,
        stamp: stamp.clone(),
    });
    proxy.set_commands(agent::driver(Arc::clone(&proxy), Arc::clone(&scope)));
    tokio::spawn(inside::mcp::notifications(Arc::clone(&proxy)));

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
            Ok(channel_request::Frame::Schema) => {
                tokio::spawn(program::schema(proxy, scope, channel));
            }
            Ok(channel_request::Frame::Enqueue(request)) => {
                tokio::spawn(agent::enqueue(proxy, scope, channel, request.prompt));
            }
            Ok(channel_request::Frame::Dequeue) => {
                tokio::spawn(agent::dequeue(proxy, scope, channel));
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

/// A second begin on a connection that had one.
pub(super) fn begun() -> Error {
    Error(serde_json::json!({
        "kind": "begun",
        "error": "this connection has begun",
    }))
}
