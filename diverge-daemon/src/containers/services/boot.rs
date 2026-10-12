//! Every service brought up at the daemon's start.

use std::sync::Arc;

use super::{held, keep, retire};
use crate::daemon::Daemon;
use crate::store::{self, services};

/// At the daemon's start: every service on record with an agent
/// attached is kept — its supervisor starts its container — and every
/// one with none is retired, since nothing holds it and nothing will.
pub async fn boot(daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let (kept, retired) = {
        let mut conn = daemon.store.acquire().await?;
        let mut kept = Vec::new();
        let mut retired = Vec::new();
        for service in services::all(&mut conn).await? {
            if services::attachments::attached(&mut conn, service.id).await?.is_empty() {
                retired.push(service.id);
            } else {
                kept.push(service.id);
            }
        }
        (kept, retired)
    };
    for id in kept {
        keep(daemon, id).await;
        held(daemon, id).await;
    }
    for id in retired {
        retire(daemon, id).await;
    }
    Ok(())
}
