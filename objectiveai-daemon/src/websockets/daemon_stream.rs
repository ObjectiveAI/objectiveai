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
//! - **Consumer side** — an [`axum`] HTTP server bound to the
//!   daemon's configured `address:port`. The broadcast lives on the
//!   `/listen` SSE route: every client that connects immediately
//!   begins receiving future frames; it is a pure push channel. The
//!   sibling `/execute` route ([`crate::websockets::daemon_execute`])
//!   runs commands in-process, one POST per command — its SSE streams
//!   never carry broadcast frames.
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
    /// `/laboratories/{id}` +
    /// `/laboratories/{id}/filetree` routes.
    pub(crate) labs_hub: crate::websockets::websocket_laboratories::LaboratoriesHub,
}

/// Serve the daemon's HTTP API on `listener`:
///
/// - **`GET /listen`** — the broadcast SSE: each client receives every
///   future frame. Pure push.
/// - **`POST /execute`** — request-per-command execution
///   ([`crate::websockets::daemon_execute`]): the client's request runs
///   in-process against `ctx`, and its items stream back on that
///   response only — never onto the broadcast. (The run's tee still
///   lands on `/listen` like any other CLI activity, via the producer
///   socket.)
/// - **`GET /agents/instances/*`, `/laboratories/*`** — the live SSE
///   watcher routes.
/// - **`/laboratory`** — the ONE WebSocket: the bidirectional
///   laboratory-host channel
///   ([`crate::websockets::websocket_laboratory`]).
///
/// Every HTTP route authenticates by the `X-OBJECTIVEAI-SIGNATURE`
/// header ([`crate::websockets::daemon_auth::authenticate_header`],
/// 401 on a missing/invalid signature when `secret` is `Some`); the
/// `/laboratory` WebSocket keeps the first-message `AuthEnvelope`
/// preamble. Returns the serve task's handle.
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
        .route("/listen", axum::routing::get(listen_handler))
        .route(
            "/execute",
            axum::routing::post(crate::websockets::daemon_execute::execute_handler),
        )
        .route(
            "/agents/instances/list",
            axum::routing::get(crate::websockets::websocket_agents::agents_handler),
        )
        // Wildcard ({*aih} — AIHs contain `/`). The literal `list`
        // route above takes matching priority; axum 0.8 permits the
        // overlap (a true conflict would panic right here at daemon
        // boot, so a clean boot is the regression check).
        .route(
            "/agents/instances/{*aih}",
            axum::routing::get(
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
            axum::routing::get(
                crate::websockets::websocket_laboratories::laboratories_handler,
            ),
        )
        // Single-segment `{id}` under the literal `list` route —
        // laboratory ids forbid `/`, so the id is one segment and
        // `/laboratories/{id}/filetree` is an unambiguous sibling.
        .route(
            "/laboratories/{id}",
            axum::routing::get(
                crate::websockets::websocket_laboratories::laboratory_instance_handler,
            ),
        )
        .route(
            "/laboratories/{id}/filetree",
            axum::routing::get(
                crate::websockets::websocket_laboratories::laboratory_filetree_handler,
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
        })
        // CORS, permissive — mirrors objectiveai-api. The viewer's
        // webview fetches these routes cross-origin (its page origin is
        // never the daemon's), and a fetch+SSE response without CORS
        // headers is opaque to a browser: the preflight 405s and the GET
        // is unreadable. Auth is the `X-OBJECTIVEAI-SIGNATURE` header
        // (never cookies), so any-origin is safe here.
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any)
                .expose_headers(tower_http::cors::Any),
        );
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    })
}

/// `GET /listen`: header-auth, then an SSE stream of every broadcast
/// frame. Pure server→client push (the daemon's activity tee); the
/// client never sends anything.
async fn listen_handler(
    axum::extract::State(state): axum::extract::State<DaemonWsState>,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if !crate::websockets::daemon_auth::authenticate_header(&headers, state.secret.as_ref()) {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    }
    axum::response::sse::Sse::new(listen_stream(state.tx))
        .keep_alive(axum::response::sse::KeepAlive::default())
        .into_response()
}

/// Forward every broadcast frame as an SSE event until the client drops
/// the stream. A `Lagged` broadcast receiver (slow client) drops missed
/// frames and keeps going; `Closed` ends the stream.
fn listen_stream(
    tx: broadcast::Sender<String>,
) -> impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>> {
    use axum::response::sse::Event;
    async_stream::stream! {
        let mut rx = tx.subscribe();
        loop {
            match rx.recv().await {
                Ok(frame) => yield Ok(Event::default().data(frame)),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    }
}

