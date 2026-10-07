//! Held, and in use.

use diverge_sdk::daemon::reference;
use sqlx::PgConnection;

use super::mounters;
use crate::daemon::Daemon;
use crate::store;

/// Whether the volume is held now: a running container has it, or a
/// download, an upload or a transfer of the daemon's is on it. What
/// an edit, a stat, a filetree, a download, an upload and a transfer
/// answer `Held` for, and ask about again once it is free.
pub async fn held(daemon: &Daemon, volume: &reference::Volume) -> bool {
    daemon.live.volume_held(volume).await
}

/// Whether the volume is in use: some agent or tool names it in its
/// mounts, running or not, or an operation is on it. What a delete
/// answers `InUse` for.
pub async fn in_use(conn: &mut PgConnection, daemon: &Daemon, volume: &reference::Volume) -> Result<bool, store::Error> {
    if daemon.live.volume_operating(volume).await {
        return Ok(true);
    }
    let (agents, tools) = mounters(conn, volume).await?;
    Ok(!agents.is_empty() || !tools.is_empty())
}
