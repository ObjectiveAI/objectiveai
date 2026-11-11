//! An agent container's begin: the arguments registered, the queue's
//! driver started, the family's channels served.

use std::sync::Arc;

use diverge_sdk::container_proxy::outside::endpoints::agents::begin::client::{channel_request, request};
use diverge_sdk::container_proxy::outside::endpoints::agents::begin::server::response;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::error::Error;
use tokio::task::JoinSet;

use super::Family;
use crate::encode::encoded;
use crate::proxy::{Begun, Proxy};
use crate::{agent, info, inside, program};

/// Serve the begin for the connection's life.
///
/// A second begin on the connection is `Error`, then the finish, and
/// the first goes on. Otherwise the arguments and the template are
/// registered with the program's server — its refusal is the `Error`,
/// in its own words, and the connection has not begun — and then
/// `Begun` goes out carrying the dependencies the program answered
/// with and the secrets it takes, by name and mode, the permissions
/// it answered with dropped, since an agent asks the daemon nothing;
/// the declaration
/// is kept first, since the loop's end reads it, and the
/// driver's gate is shut from here until the server's first feed —
/// its second when the program registered anything, since the first
/// is then the provider's at the deploy. The scope is published to
/// every surface inside the container, the queue's driver and the
/// resident notifications ask are started, and every channel the
/// server opens on the scope is served on a task of its own — the
/// image's `/DIVERGE.md` read from the root at every ask, the program
/// asked nothing; a channel this end cannot read is finished with
/// nothing. No daemon
/// connection is among them: an agent container holds no account and
/// asks the daemon nothing, so the server never opens one, and
/// `/daemon` closes on an agent container at once. The loop is raced
/// against the proxy ending the run — a collect that failed, its
/// `Error` on the main stream already — and finishes the scope then;
/// this loop alone may, since reading the inbox holds its lock, and
/// a finish from any other task would wait on it forever. Otherwise
/// the scope is the connection's life.
///
/// Every task it starts is its own — the channels', and for an agent
/// the resident notifications ask — held in one set: those that ended
/// are reaped before the next channel is taken, so the set holds what
/// is in flight and no more; the finish is followed by the rest,
/// awaited, each ending with its channel; and the connection ending,
/// which aborts this task, aborts every one of them with it.
pub async fn agents(proxy: Arc<Proxy>, scope: ScopeHandle, frame: request::Frame) {
    if !proxy.claim_begin().await {
        refuse(&scope, begun()).await;
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
    let begun = response::Registered {
        dependencies: registered.dependencies,
        secrets: registered.secrets,
    };
    let registering = !begun.dependencies.is_empty() || !begun.secrets.is_empty();
    if let Some(payload) = encoded(&response::Frame::Begun(begun)) {
        scope.send_response(&payload).await;
    }
    proxy.publish(Begun {
        scope: Arc::clone(&scope),
        family: Family::Agents,
    });
    proxy.set_commands(agent::driver(Arc::clone(&proxy), Arc::clone(&scope), registering));
    let mut tasks = JoinSet::new();
    tasks.spawn(inside::mcp::notifications(Arc::clone(&proxy)));

    loop {
        // Finished ones, so the set does not grow for the life of the
        // connection.
        while tasks.try_join_next().is_some() {}
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
            Ok(channel_request::Frame::Postgres(request)) => {
                tasks.spawn(inside::postgres::attach(proxy, scope, channel, request.connection_id));
            }
            Ok(channel_request::Frame::Info) => {
                tasks.spawn(info::info(scope, channel));
            }
            Ok(channel_request::Frame::Enqueue(request)) => {
                tasks.spawn(agent::enqueue(proxy, scope, channel, request.messages));
            }
            Ok(channel_request::Frame::Secrets(values)) => {
                tasks.spawn(agent::feed(proxy, scope, channel, values));
            }
            Err(_) => {
                tasks.spawn(async move { scope.send_channel_response_finish(channel).await });
            }
        }
    }
    scope.send_response_finish().await;
    while tasks.join_next().await.is_some() {}
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
