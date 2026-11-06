//! Every caller's session, by the caller.

use std::collections::HashMap;
use std::sync::Arc;

use diverge_sdk::shared::mcp::attestation::Caller;
use diverge_sdk::shared::mcp::session;
use tokio::sync::Mutex;

use super::Session;

/// The sessions open, one per caller by its
/// [`key`](diverge_sdk::shared::mcp::session::key). The rules:
///
/// 1. A caller's first exchange makes its session, not yet dialled;
///    every exchange after finds the same one.
/// 2. [`close`](Self::close) removes a session by its key, and closes
///    it: its client cancelled, nothing dialled on it again. A holder
///    still in an exchange finishes it on the client it holds; a
///    notifications channel on it ends with its channel.
/// 3. Every session is removed when a new one is made if nothing but
///    the map holds it and nothing of it is live — no client held, or
///    one whose transport has died: a session so idle is a fresh one
///    in all but its allocation, and the caller's next exchange makes
///    it again. So the map holds the sessions in use or with a live
///    client, and those idle since the last new one, never every
///    caller ever seen.
#[derive(Default)]
pub struct Sessions {
    map: Mutex<HashMap<String, Arc<Session>>>,
}

impl Sessions {
    /// The caller's session: the one open, or a new one, not yet
    /// dialled, kept from now on (rule 1) — the idle ones dropped
    /// first when it is new (rule 3).
    pub async fn get(&self, caller: &Caller) -> Arc<Session> {
        let key = session::key(caller);
        let mut map = self.map.lock().await;
        if let Some(session) = map.get(&key) {
            return Arc::clone(session);
        }
        // Held only by the map, under the map's lock, so nothing can
        // take it between the look and the removal.
        map.retain(|_, session| Arc::strong_count(session) > 1 || session.idle() != Some(true));
        let session = Arc::new(Session::new(caller.clone()));
        map.insert(key, Arc::clone(&session));
        session
    }

    /// Remove the session under `key` and close it (rule 2). A key
    /// with no session is nothing.
    pub async fn close(&self, key: &str) {
        let session = self.map.lock().await.remove(key);
        if let Some(session) = session {
            session.close().await;
        }
    }
}
