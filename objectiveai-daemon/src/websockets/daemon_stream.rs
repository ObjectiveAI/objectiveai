//! The resident daemon's broadcast hub.
//!
//! Two listeners share one [`broadcast`] channel of already-serialized
//! JSON frames:
//!
//! - **Producer side** — a fixed-name local socket (`<state>/socks/daemon.sock`
//!   on Unix; a namespaced pipe on Windows). A producer connects, streams
//!   its agent/plugin **context** object, then one CLI **request** line,
//!   then that request's CLI **response** lines (newline-delimited JSON,
//!   no ack), then closes. `interprocess` inserts no framing of its own,
//!   so the trailing `\n` is the only delimiter — the same wire shape as
//!   [`crate::websockets::mcp_listener`].
//! - **Consumer side** — an [`axum`] WebSocket server bound to the
//!   daemon's configured `address:port`. The broadcast lives on the
//!   `/listen` route: every client that connects immediately begins
//!   receiving future frames; it is a pure push channel (inbound
//!   messages are ignored except to notice the client closing). The
//!   sibling `/execute` route ([`crate::websockets::daemon_execute`])
//!   runs commands in-process, one connection per command — its
//!   streams never carry broadcast frames.
//!
//! Each producer connection is assigned a fresh `id`. The request is
//! wrapped as the SDK's generic `ListenerRequest<T>` shape
//! (`{…context, id, value}` — the producer's context fields stamped
//! alongside `id`); every following item as the bare
//! `ListenerResponse<T>` `{id, value}` wrapper (no type tag — a
//! consumer already knows how to deserialize each id's items from its
//! opening request); and when the producer's feed closes, one
//! [`ListenerEnd`] (`{id, end: true}`) marks that stream complete.
//! The `id` is the whole routing story: it demultiplexes concurrent
//! producer streams and discriminates the frame shapes (terminator by
//! `end: true`; response when the id is already announced; request
//! otherwise).
//!
//! Frames are constructed raw — the underlying items stay opaque
//! [`serde_json::Value`]s on the wire, so the hub is forward-compatible
//! with command shapes it predates.
//!
//! Broadcast items are always the PRE-transform, leaf-typed response
//! items: the producer tee sits below the executor's jq/python
//! transform adapters, so `/listen` consumers see every execution's
//! typed activity even when the command's own output is transformed.

use tokio::sync::broadcast;

/// Shared state for the daemon's WebSocket routes: the broadcast
/// sender `/listen` subscribers drain, the resident
/// [`crate::context::Context`] that `/execute` runs commands against,
/// and the optional secret every connection's auth preamble is
/// verified against.
#[derive(Clone)]
pub(crate) struct DaemonWsState {
    pub(crate) tx: broadcast::Sender<String>,
    pub(crate) ctx: crate::context::Context,
    pub(crate) secret: Option<std::sync::Arc<String>>,
    /// The live agent-status registry backing the `/agents/instances/list` route.
    pub(crate) active: crate::websockets::websocket_agents::ActiveAgents,
    /// The live-conversation hub backing the `/agents/instances/{*aih}`
    /// route.
    pub(crate) conversations: crate::websockets::websocket_agent_instance::ConversationHub,
    /// The connected-laboratory registry backing the `/laboratory`
    /// route and `laboratories.sock`.
    pub(crate) laboratories: crate::websockets::websocket_laboratory::LaboratoryRegistry,
    /// The live-laboratories hub backing the `/laboratories/list` +
    /// `/laboratories/{*id}` routes.
    pub(crate) labs_hub: crate::websockets::websocket_laboratories::LaboratoriesHub,
}

/// Serve the daemon's WebSocket API on `listener`. Two routes, strictly
/// separated:
///
/// - **`/listen`** — the broadcast: each client receives every future
///   frame. Pure push; after the auth preamble, inbound messages are
///   never treated as requests.
/// - **`/execute`** — connection-per-command execution
///   ([`crate::websockets::daemon_execute`]): the client's request runs
///   in-process against `ctx`, and its items stream back on that socket
///   only — never onto the broadcast. (The run's tee still lands on
///   `/listen` like any other CLI activity, via the producer socket.)
/// - **`/agents/instances/list`** — the live agent-status stream
///   ([`crate::websockets::websocket_agents`]): a connect-time snapshot of
///   every agent, then `Activated`/`Deactivated` deltas driven by
///   AIH-lockfile release. Backed by `state.active`.
///
/// EVERY connection on both routes starts with the first-message auth
/// preamble ([`crate::websockets::daemon_auth::authenticate`]): the
/// first text frame must be the SDK `AuthEnvelope`. When `secret` is
/// `Some`, a missing/invalid signature closes the connection; when
/// `None`, the envelope is consumed and ignored. Headers are never
/// used. Returns the serve task's handle.
pub fn serve_ws(
    listener: tokio::net::TcpListener,
    tx: broadcast::Sender<String>,
    secret: Option<std::sync::Arc<String>>,
    ctx: crate::context::Context,
    active: crate::websockets::websocket_agents::ActiveAgents,
    conversations: crate::websockets::websocket_agent_instance::ConversationHub,
    laboratories: crate::websockets::websocket_laboratory::LaboratoryRegistry,
    labs_hub: crate::websockets::websocket_laboratories::LaboratoriesHub,
) -> tokio::task::JoinHandle<()> {
    let app = axum::Router::new()
        .route("/listen", axum::routing::any(listen_handler))
        .route(
            "/execute",
            axum::routing::any(crate::websockets::daemon_execute::execute_handler),
        )
        .route(
            "/agents/instances/list",
            axum::routing::any(crate::websockets::websocket_agents::agents_handler),
        )
        // Wildcard ({*aih} — AIHs contain `/`). The literal `list`
        // route above takes matching priority; axum 0.8 permits the
        // overlap (a true conflict would panic right here at daemon
        // boot, so a clean boot is the regression check).
        .route(
            "/agents/instances/{*aih}",
            axum::routing::any(
                crate::websockets::websocket_agent_instance::instance_handler,
            ),
        )
        // Laboratory managers dial in here: Identify frame first,
        // auth preamble second, then correlated request/response.
        .route(
            "/laboratory",
            axum::routing::any(
                crate::websockets::websocket_laboratory::laboratory_handler,
            ),
        )
        .route(
            "/laboratories/list",
            axum::routing::any(
                crate::websockets::websocket_laboratories::laboratories_handler,
            ),
        )
        // Wildcard ({*id}) under the literal `list` route — the same
        // proven axum-0.8 overlap as `/agents/instances/*` above.
        .route(
            "/laboratories/{*id}",
            axum::routing::any(
                crate::websockets::websocket_laboratories::laboratory_instance_handler,
            ),
        )
        .with_state(DaemonWsState {
            tx,
            ctx,
            secret,
            active,
            conversations,
            laboratories,
            labs_hub,
        });
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    })
}

/// `/listen`: upgrade to WebSocket, consume the auth preamble, and
/// pump broadcast frames.
async fn listen_handler(
    axum::extract::State(state): axum::extract::State<DaemonWsState>,
    ws: axum::extract::ws::WebSocketUpgrade,
) -> axum::response::Response {
    ws.on_upgrade(move |mut socket| async move {
        if !crate::websockets::daemon_auth::authenticate(&mut socket, state.secret.as_ref()).await
        {
            return;
        }
        pump(socket, state.tx).await;
    })
}

/// Forward every broadcast frame to one client until it disconnects.
/// Pure push: inbound frames are read only to notice the close. A
/// `Lagged` broadcast receiver (slow client) drops missed frames and
/// keeps going.
async fn pump(mut socket: axum::extract::ws::WebSocket, tx: broadcast::Sender<String>) {
    use axum::extract::ws::Message;
    let mut rx = tx.subscribe();
    loop {
        tokio::select! {
            received = rx.recv() => match received {
                Ok(frame) => {
                    if socket.send(Message::Text(frame.into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            inbound = socket.recv() => match inbound {
                // Client closed or errored.
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                // Ignore any other inbound message.
                Some(Ok(_)) => {}
            },
        }
    }
}

