//! The providers a container names, checked to be the daemon's.

use diverge_sdk::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};
use diverge_sdk::daemon::agent_log::Identity;
use sqlx::PgConnection;

use super::Checked;
use crate::daemon::Daemon;
use crate::store;

/// Every provider the create or the edit names — the pin, and the
/// provider of each cross-provider mount — one with a record: `Error`
/// naming the first that is not, else nothing. The rules:
///
/// 1. **Checked in the caller's transaction.** Each provider named is
///    read from the store, its row held shared until the caller
///    commits — see [`share`](store::providers::share).
/// 2. **No stranded pin.** A delete of a provider holds its row for
///    update, so it waits for every create and edit naming it to
///    commit, and then finds their pins and answers `in_use`; a create
///    or an edit that comes after a delete finds no record.
/// 3. **Each once.** A provider named twice is read once.
pub async fn providers(conn: &mut PgConnection, pinned: Option<&Provider>, file_mounts: &[FuseMount], directory_mounts: &[FuseMount]) -> Result<Checked<()>, store::Error> {
    let named = pinned
        .map(|provider| &provider.identity)
        .into_iter()
        .chain(file_mounts.iter().map(|mount| &mount.provider))
        .chain(directory_mounts.iter().map(|mount| &mount.provider));
    let mut seen: Vec<&Identity> = Vec::new();
    for identity in named {
        if seen.contains(&identity) {
            continue;
        }
        seen.push(identity);
        if !store::providers::share(conn, identity).await? {
            return Ok(Checked::Error(format!("no provider is known as {identity}")));
        }
    }
    Ok(Checked::Ok(()))
}

/// Whether the daemon knows a provider by the identity: one it keeps a
/// record of, enabled or not.
pub fn on_record(daemon: &Daemon, identity: &Identity) -> bool {
    daemon.providers.accepts(identity)
}
