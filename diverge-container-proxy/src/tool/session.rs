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
/// re-dialled when its transport has died, and what the server sends
/// on it.
pub struct Session {
    caller: Caller,
    running: Mutex<Option<Arc<RunningService<RoleClient, Handler>>>>,
    notifications: broadcast::Sender<ServerNotification>,
}

impl Session {
    /// The caller's session, not yet dialled.
    pub fn new(caller: Caller) -> Self {
        let (notifications, _) = broadcast::channel(NOTIFICATIONS_CAPACITY);
        Session {
            caller,
            running: Mutex::new(None),
            notifications,
        }
    }

    /// The live client: the one held, or a fresh dial — initialized
    /// with the caller under `_meta`, and ready — when none is held or
    /// the held one's transport is gone. A dial that fails is the
    /// exchange's error, and nothing is held.
    pub async fn client(&self) -> Result<Arc<RunningService<RoleClient, Handler>>, ErrorData> {
        let mut running = self.running.lock().await;
        if let Some(client) = running.as_ref()
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
        *running = Some(Arc::clone(&client));
        Ok(client)
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
