//! The daemon's `/user` channel — USER REQUESTS: `user request`
//! commands broadcast to every connected user stream; the first
//! ACCEPTED reply wins.
//!
//! [`UserHub`] is a per-connection registry with TRACKED delivery
//! (the laboratory-registry model, not a fire-and-forget broadcast):
//!
//! - Every `GET /user` connection registers an id + its own mpsc
//!   sender. On registration it is REPLAYED every PENDING request —
//!   never settled or timed-out ones.
//! - Each pending request remembers exactly WHICH connection ids it
//!   was delivered to (`notified`). The settlement ack and the
//!   timeout notice go only to those connections — a stream that
//!   never saw the request never hears about its end.
//! - Replies arbitrate on the request's `settle` mutex: the FIRST
//!   reply takes the oneshot and wins; the daemon then acks all
//!   notified connections with [`UserEvent::Settled`] so they know no
//!   further reply is possible.
//! - The originating command's drop guard calls [`UserHub::abandon`]
//!   whenever the wait ends without a winner — base `--timeout`
//!   abort, caller disconnect, any error — which notifies the
//!   notified connections with [`UserEvent::TimedOut`]. Zero
//!   connected streams still leaves the request pending for later
//!   connections to replay.

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::response::sse::{Event, Sse};
use dashmap::DashMap;
use objectiveai_sdk::cli::command::AgentArguments;
use objectiveai_sdk::cli::user_listener::{
    UserEvent, UserReply, UserReplyOutcome, UserRequest,
};
use tokio::sync::{mpsc, oneshot};

/// One pending user request held by the hub.
struct PendingUserRequest {
    /// The pre-serialized [`UserEvent::Request`] frame — built once,
    /// sent to every current and future connection.
    frame: String,
    /// Connection ids this request was delivered to — the exact
    /// audience for its settlement/timeout notice. Guarded by a sync
    /// mutex (tiny critical sections, never held across an await).
    notified: std::sync::Mutex<HashSet<u64>>,
    /// The winner slot AND the reply-arbitration lock: replies
    /// serialize on this mutex; a taken (`None`) sender means the
    /// request is already settled.
    settle: tokio::sync::Mutex<Option<oneshot::Sender<(AgentArguments, serde_json::Value)>>>,
}

impl Default for UserHub {
    fn default() -> Self {
        Self::new()
    }
}

/// The user-requests hub — see the module docs. Clone-shared.
#[derive(Clone)]
pub struct UserHub {
    connections: Arc<DashMap<u64, mpsc::UnboundedSender<String>>>,
    next_connection_id: Arc<AtomicU64>,
    pending: Arc<DashMap<String, Arc<PendingUserRequest>>>,
}

impl UserHub {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
            next_connection_id: Arc::new(AtomicU64::new(1)),
            pending: Arc::new(DashMap::new()),
        }
    }

    /// Register a `GET /user` connection: allocate its id + channel,
    /// then replay every pending request (marking this connection
    /// notified per entry). Registration precedes the replay so a
    /// settle racing it still reaches this connection — the notified
    /// insert and the settle's notified read serialize on each
    /// entry's `notified` mutex.
    fn register_connection(&self) -> (u64, mpsc::UnboundedReceiver<String>) {
        let id = self.next_connection_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded_channel();
        self.connections.insert(id, tx.clone());
        for entry in self.pending.iter() {
            let request = entry.value();
            let mut notified = request.notified.lock().expect("notified lock");
            if notified.insert(id) {
                let _ = tx.send(request.frame.clone());
            }
        }
        // The caught-up marker: replay done — an event a consumer can
        // AWAIT instead of sleeping to "prove" absence.
        let live = serde_json::to_string(&UserEvent::Live).expect("UserEvent serializes");
        let _ = tx.send(live);
        (id, rx)
    }

    /// Drop a closed connection. Stale ids left in `notified` sets
    /// are harmless — their sends go nowhere.
    fn unregister_connection(&self, id: u64) {
        self.connections.remove(&id);
    }

    /// Create a pending request and fan it out to every current
    /// connection. Returns the receiver the originating command
    /// awaits; the hub-minted id is `request.id`.
    pub fn create(
        &self,
        request: &UserRequest,
    ) -> oneshot::Receiver<(AgentArguments, serde_json::Value)> {
        let frame = serde_json::to_string(&UserEvent::Request {
            request: request.clone(),
        })
        .expect("UserEvent serializes");
        let (settle_tx, settle_rx) = oneshot::channel();
        let entry = Arc::new(PendingUserRequest {
            frame,
            notified: std::sync::Mutex::new(HashSet::new()),
            settle: tokio::sync::Mutex::new(Some(settle_tx)),
        });
        self.pending.insert(request.id.clone(), Arc::clone(&entry));
        for connection in self.connections.iter() {
            let mut notified = entry.notified.lock().expect("notified lock");
            if notified.insert(*connection.key()) {
                let _ = connection.value().send(entry.frame.clone());
            }
        }
        settle_rx
    }

    /// One reply for `id`. Serializes on the entry's `settle` mutex so
    /// two racing replies can never both win — the first to take the
    /// oneshot is the winner; everyone after sees `Settled`.
    pub async fn reply(
        &self,
        id: &str,
        identity: AgentArguments,
        reply: serde_json::Value,
    ) -> UserReplyOutcome {
        let Some(entry) = self.pending.get(id).map(|e| Arc::clone(e.value())) else {
            return UserReplyOutcome::NotFound;
        };
        let mut settle = entry.settle.lock().await;
        let Some(winner) = settle.take() else {
            // Already won by a racer (the entry leaves `pending`
            // right after).
            return UserReplyOutcome::Settled;
        };
        drop(settle);
        self.pending.remove(id);
        let _ = winner.send((identity.clone(), reply));
        self.notify(
            &entry,
            &UserEvent::Settled {
                id: id.to_string(),
                identity,
            },
        );
        UserReplyOutcome::Accepted
    }

    /// End a request WITHOUT a winner (the originating command's wait
    /// ended — timeout, disconnect, error): drop it from `pending`
    /// and send [`UserEvent::TimedOut`] to the connections that saw
    /// it. Idempotent; a request that already settled is a no-op.
    pub fn abandon(&self, id: &str) {
        let Some((_, entry)) = self.pending.remove(id) else {
            return;
        };
        self.notify(&entry, &UserEvent::TimedOut { id: id.to_string() });
    }

    /// Send one event to exactly the connections that saw `entry`.
    fn notify(&self, entry: &PendingUserRequest, event: &UserEvent) {
        let frame = serde_json::to_string(event).expect("UserEvent serializes");
        let notified = entry.notified.lock().expect("notified lock");
        for id in notified.iter() {
            if let Some(connection) = self.connections.get(id) {
                let _ = connection.send(frame.clone());
            }
        }
    }
}

/// RAII unregistration: dropping the SSE stream (client gone) is the
/// unregister signal — the filetree-watch guard pattern.
struct ConnectionGuard {
    hub: UserHub,
    id: u64,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.hub.unregister_connection(self.id);
    }
}

/// `GET /user`: header-auth, then an SSE stream of [`UserEvent`]s —
/// the pending replay first, live traffic after.
pub(crate) async fn user_handler(
    axum::extract::State(state): axum::extract::State<
        crate::http::daemon_stream::DaemonHttpState,
    >,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if !crate::http::daemon_auth::authenticate_header(
        &headers,
        state.global.auth_secret().as_ref(),
    ) {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    }
    let hub = state.user.clone();
    let (id, mut rx) = hub.register_connection();
    let stream = async_stream::stream! {
        let _guard = ConnectionGuard { hub, id };
        while let Some(frame) = rx.recv().await {
            yield Ok::<_, std::convert::Infallible>(Event::default().data(frame));
        }
    };
    Sse::new(stream).into_response()
}

/// `POST /user/{id}/reply`: header-auth, replier identity from the
/// `X-OBJECTIVEAI-*` headers, [`UserReply`] body. Always answers with
/// a [`UserReplyOutcome`] JSON body; the status code mirrors it
/// (200 accepted / 422 rejected / 409 settled / 404 unknown).
pub(crate) async fn user_reply_handler(
    axum::extract::State(state): axum::extract::State<
        crate::http::daemon_stream::DaemonHttpState,
    >,
    axum::extract::Path(id): axum::extract::Path<String>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if !crate::http::daemon_auth::authenticate_header(
        &headers,
        state.global.auth_secret().as_ref(),
    ) {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    }
    let reply: UserReply = match serde_json::from_slice(&body) {
        Ok(reply) => reply,
        Err(e) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                format!("user reply body: {e}"),
            )
                .into_response();
        }
    };
    let identity = crate::http::daemon_execute::agent_arguments(&headers);
    let outcome = state.user.reply(&id, identity, reply.reply).await;
    let status = match &outcome {
        UserReplyOutcome::Accepted => axum::http::StatusCode::OK,
        UserReplyOutcome::Settled => axum::http::StatusCode::CONFLICT,
        UserReplyOutcome::NotFound => axum::http::StatusCode::NOT_FOUND,
    };
    let body = serde_json::to_string(&outcome).expect("UserReplyOutcome serializes");
    (status, body).into_response()
}
