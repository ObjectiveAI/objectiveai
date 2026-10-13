//! Every service brought up at the daemon's start.

use std::sync::Arc;

use super::{held, keep};
use crate::daemon::Daemon;
use crate::store::{self, dependency_tools};

/// At the daemon's start: every service on record with an agent
/// attached is kept — its supervisor starts its container. One with
/// none is left as it is: nothing holds it, and the next registration
/// that attaches to it keeps it up.
pub async fn boot(daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let kept = {
        let mut conn = daemon.store.acquire().await?;
        let mut kept = Vec::new();
        for tool in dependency_tools::all(&mut conn).await? {
            if tool.is_service() && !dependency_tools::attachments::attached(&mut conn, tool.id).await?.is_empty() {
                kept.push(tool.id);
            }
        }
        kept
    };
    for id in kept {
        keep(daemon, id).await;
        held(daemon, id).await;
    }
    Ok(())
}
