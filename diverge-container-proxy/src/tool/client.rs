//! The one MCP client to the tool container's server.

use std::sync::Arc;

use diverge_container_proxy_sdk::agent;
use rmcp::model::{ClientInfo, ClientResult, ServerNotification, ServerRequest};
use rmcp::service::{NotificationContext, RequestContext, RunningService, ServiceExt as _};
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::{ErrorData, RoleClient, Service, ServiceError};
use tokio::sync::{Mutex, broadcast};

/// The server's URL, for every path.
fn url() -> String {
    format!("http://127.0.0.1:{}/mcp", agent::port())
}

/// How many notifications a slow `/tool/notifications` may fall
/// behind before it loses the oldest. A subscriber that lags is told
/// so and goes on; nothing waits on it.
const NOTIFICATIONS_CAPACITY: usize = 256;

/// One client, dialled when first needed and re-dialled when its
/// transport has died, and the notifications it hears.
pub struct Tool {
    running: Mutex<Option<Arc<RunningService<RoleClient, Handler>>>>,
    notifications: broadcast::Sender<ServerNotification>,
}

impl Tool {
    pub fn new() -> Self {
        let (notifications, _) = broadcast::channel(NOTIFICATIONS_CAPACITY);
        Tool {
            running: Mutex::new(None),
            notifications,
        }
    }

    /// The live client: the one held, or a fresh dial — initialized
    /// and ready — when none is held or the held one's transport is
    /// gone. A dial that fails is the exchange's error, and nothing
    /// is held.
    pub async fn client(&self) -> Result<Arc<RunningService<RoleClient, Handler>>, ErrorData> {
        let mut running = self.running.lock().await;
        if let Some(client) = running.as_ref() {
            if !client.is_transport_closed() {
                return Ok(Arc::clone(client));
            }
        }
        let handler = Handler {
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

    /// Everything the server says from now on.
    pub fn subscribe(&self) -> broadcast::Receiver<ServerNotification> {
        self.notifications.subscribe()
    }
}

/// The client's own side: it answers the server's ping and nothing
/// else, and hands every notification to whoever subscribed.
pub struct Handler {
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

    fn get_info(&self) -> ClientInfo {
        ClientInfo::default()
    }
}

/// A call that never reached the server, or that the transport lost,
/// in the exchange's own vocabulary.
pub fn refused(reason: String) -> ErrorData {
    ErrorData::internal_error(reason, None)
}

/// What a call came to: the server's own error travels as itself,
/// code and all; the client's failures are internal errors.
pub fn outcome<T>(result: Result<T, ServiceError>) -> Result<T, ErrorData> {
    match result {
        Ok(value) => Ok(value),
        Err(ServiceError::McpError(error)) => Err(error),
        Err(error) => Err(refused(error.to_string())),
    }
}
