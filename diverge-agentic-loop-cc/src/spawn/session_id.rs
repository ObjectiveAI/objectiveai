//! The session's id: the one thing this container caches.

use tokio::sync::Mutex;

/// The session Claude Code is running, or `None` until it is known.
///
/// Known once, and then for the program's life: the conversation IS
/// the files under the config dir, and this program is the same
/// system across every run it serves, so the id that names those
/// files never changes and the files never need writing again — a
/// resumed run launches `claude --resume <id>` and finds them where
/// Claude Code itself has been appending to them. Set by whichever
/// comes first: the run handler, when it loaded the continuation
/// from the row and wrote its files; or the reader, from the first
/// record naming the session. Never cleared, and never changed once
/// set — a later record or a later run naming it again agrees with
/// it by construction. The harvest at every run's end is keyed by
/// it.
static SESSION_ID: Mutex<Option<String>> = Mutex::const_new(None);

/// The session id, if it is known.
pub async fn session_id() -> Option<String> {
    SESSION_ID.lock().await.clone()
}

/// Name the session, if it is not named yet. A second naming is
/// ignored: the first one stands for the program's life.
pub async fn remember(session_id: String) {
    let mut known = SESSION_ID.lock().await;
    if known.is_none() {
        *known = Some(session_id);
    }
}
