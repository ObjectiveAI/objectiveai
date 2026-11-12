//! The daemon, as the tool reaches it: for the one thing it asks, a
//! login.

use std::sync::Arc;

use diverge_sdk::container_proxy::inside::Client;
use diverge_sdk::wire::client::handle::Handle;
use tokio::sync::Mutex;

use crate::failure::Failure;

/// The connection to the daemon, through the proxy, served under the
/// tool's own grants — the two logins: dialled on the first use, kept,
/// and dialled again when an ask finds it gone.
#[derive(Default)]
pub struct Daemon {
    handle: Arc<Mutex<Option<Handle>>>,
}

impl Daemon {
    /// The handle: the one held, or a new one dialled.
    pub async fn handle(&self, client: &Client) -> Result<Handle, Failure> {
        let mut held = self.handle.lock().await;
        if let Some(handle) = held.as_ref() {
            return Ok(handle.clone());
        }
        let handle = client.daemon().await.map_err(|error| Failure::Daemon(error.to_string()))?;
        *held = Some(handle.clone());
        Ok(handle)
    }

    /// Forget the held connection, so the next ask dials again.
    pub async fn drop_handle(&self) {
        *self.handle.lock().await = None;
    }
}
