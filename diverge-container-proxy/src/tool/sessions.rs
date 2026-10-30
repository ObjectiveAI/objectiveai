//! Every caller's session, by the caller.

use std::collections::HashMap;
use std::sync::Arc;

use diverge_sdk::shared::mcp::attestation::Caller;
use diverge_sdk::shared::mcp::session;
use tokio::sync::Mutex;

use super::Session;

/// The sessions opened so far, one per caller by its
/// [`key`](diverge_sdk::shared::mcp::session::key).
#[derive(Default)]
pub struct Sessions {
    map: Mutex<HashMap<String, Arc<Session>>>,
}

impl Sessions {
    /// The caller's session: the one opened, or a new one, not yet
    /// dialled, kept from now on.
    pub async fn get(&self, caller: &Caller) -> Arc<Session> {
        let mut map = self.map.lock().await;
        Arc::clone(map.entry(session::key(caller)).or_insert_with(|| Arc::new(Session::new(caller.clone()))))
    }
}
