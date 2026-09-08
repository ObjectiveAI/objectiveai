//! The session's id, captured off the wire.

use tokio::sync::Mutex;

/// The session Claude Code is actually running, or `None` until a
/// record names it. The reader captures it from the first record
/// that carries one; the run handler reads it when the stream ends,
/// because the continuation harvest is keyed by it. Never cleared —
/// the id outlives the run on purpose: the harvest happens AFTER the
/// stream is over.
pub static SESSION_ID: Mutex<Option<String>> = Mutex::const_new(None);

/// The captured session id, cloned out.
pub async fn session_id() -> Option<String> {
    SESSION_ID.lock().await.clone()
}
