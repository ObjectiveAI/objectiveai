//! A service's hold, recomputed from its attachments.

use std::time::Instant;

use crate::daemon::Daemon;
use crate::store::{DependencyToolId, dependency_tools};

/// The service's attachments changed: whether any agent is attached
/// read from the records, its state told, and the moment noted as
/// the last change, which its clock counts from. A service with no
/// state — never kept, or retired — is left as it is.
pub async fn held(daemon: &Daemon, id: DependencyToolId) {
    let Some(state) = daemon.live.service_state(id).await else {
        return;
    };
    let attached = match daemon.store.acquire().await {
        Ok(mut conn) => dependency_tools::attachments::attached(&mut conn, id).await.map(|agents| !agents.is_empty()),
        Err(error) => Err(error),
    };
    // A store that cannot be read says nothing: the hold is left as it
    // was, rather than a service retired for want of an answer.
    if let Ok(attached) = attached {
        state.touched.send_replace(Instant::now());
        state.held.send_replace(attached);
    }
}
