//! One caller's MCP session to the tool container's server.

use std::sync::Arc;

use diverge_sdk::shared::mcp::attest_request;
use diverge_sdk::shared::mcp::attestation::Caller;
use rmcp::model::{ClientInfo, ClientResult, ServerNotification, ServerRequest};
use rmcp::service::{NotificationContext, RequestContext, RunningService, ServiceExt as _};
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::{ErrorData, RoleClient, Service};
use tokio::sync::{Mutex, broadcast};

use super::{refused, url};

/// How many notifications a slow notifications channel may fall
/// behind before it loses the oldest. A subscriber that lags goes on;
/// nothing waits on it.
const NOTIFICATIONS_CAPACITY: usize = 256;

/// One caller's session: its client, dialled when first needed and
/// re-dialled when its transport has died, until the session is
/// closed; and what the server sends on it.
pub struct Session {
    caller: Caller,
    running: Mutex<Running>,
    notifications: broadcast::Sender<ServerNotification>,
}

/// The client a session holds, and whether the session is closed —
/// one lock, so a dial and a close never cross.
struct Running {
    client: Option<Arc<RunningService<RoleClient, Handler>>>,
    closed: bool,
}

impl Session {
    /// The caller's session, not yet dialled.
    pub fn new(caller: Caller) -> Self {
        let (notifications, _) = broadcast::channel(NOTIFICATIONS_CAPACITY);
        Session {
            caller,
            running: Mutex::new(Running {
                client: None,
                closed: false,
            }),
            notifications,
        }
    }

    /// The live client: the one held, or a fresh dial — initialized
    /// with the caller under `_meta`, and ready — when none is held or
    /// the held one's transport is gone. A dial that fails is the
    /// exchange's error, and nothing is held. A closed session dials
    /// nothing: every exchange on it is refused.
    pub async fn client(&self) -> Result<Arc<RunningService<RoleClient, Handler>>, ErrorData> {
        let mut running = self.running.lock().await;
        if running.closed {
            return Err(refused("the session was closed".to_string()));
        }
        if let Some(client) = running.client.as_ref()
            && !client.is_transport_closed()
        {
            return Ok(Arc::clone(client));
        }
        let handler = Handler {
            caller: self.caller.clone(),
            notifications: self.notifications.clone(),
        };
        let client = handler
            .serve(StreamableHttpClientTransport::from_uri(url()))
            .await
            .map_err(|error| refused(error.to_string()))?;
        let client = Arc::new(client);
        running.client = Some(Arc::clone(&client));
        Ok(client)
    }

    /// Close the session: the client held, if any, is cancelled — its
    /// service ended and its transport with it, whoever still holds
    /// it — and dropped, and nothing is dialled on the session again.
    /// Idempotent.
    pub async fn close(&self) {
        let mut running = self.running.lock().await;
        running.closed = true;
        if let Some(client) = running.client.take() {
            client.cancellation_token().cancel();
        }
    }

    /// Whether nothing of the session's is live: no client held, or
    /// one whose transport is gone. `None` when the session is in use
    /// — its lock held — and so not idle in any sense.
    pub fn idle(&self) -> Option<bool> {
        let running = self.running.try_lock().ok()?;
        Some(running.client.as_ref().is_none_or(|client| client.is_transport_closed()))
    }

    /// Everything the server says on this session from now on.
    pub fn subscribe(&self) -> broadcast::Receiver<ServerNotification> {
        self.notifications.subscribe()
    }
}

/// The client's own side: it introduces the caller, answers the
/// server's ping and nothing else, and hands every notification to
/// whoever subscribed to this session.
pub struct Handler {
    caller: Caller,
    notifications: broadcast::Sender<ServerNotification>,
}

impl Service<RoleClient> for Handler {
    async fn handle_request(
        &self,
        request: ServerRequest,
        _context: RequestContext<RoleClient>,
    ) -> Result<ClientResult, ErrorData> {
        match request {
            ServerRequest::PingRequest(_) => Ok(ClientResult::empty(())),
            _ => Err(ErrorData::internal_error(
                "the proxy answers no server request but ping",
                None,
            )),
        }
    }

    async fn handle_notification(
        &self,
        notification: ServerNotification,
        _context: NotificationContext<RoleClient>,
    ) -> Result<(), ErrorData> {
        // Nobody listening is not a failure: a notification with no
        // subscriber is simply not heard.
        let _ = self.notifications.send(notification);
        Ok(())
    }

    /// The `initialize` the session opens with: the caller under
    /// `_meta`, as the daemon attested it.
    fn get_info(&self) -> ClientInfo {
        let mut info = ClientInfo::default();
        attest_request(&mut info, &self.caller);
        info
    }
}
