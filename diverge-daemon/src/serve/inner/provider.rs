//! The providers a container names, checked to be the configuration's.

use diverge_sdk::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};
use diverge_sdk::daemon::agent_log::Identity;

use super::Checked;
use crate::daemon::Daemon;

/// Every provider the create names — the pin, and the provider of
/// each cross-provider mount — one the configuration accepts: `Error`
/// naming the first that is not, else nothing.
pub fn providers(daemon: &Daemon, pinned: Option<&Provider>, file_mounts: &[FuseMount], directory_mounts: &[FuseMount]) -> Checked<()> {
    let named = pinned
        .map(|provider| &provider.identity)
        .into_iter()
        .chain(file_mounts.iter().map(|mount| &mount.provider))
        .chain(directory_mounts.iter().map(|mount| &mount.provider));
    for identity in named {
        if !on_record(daemon, identity) {
            return Checked::Error(format!("no provider is known as {identity}"));
        }
    }
    Checked::Ok(())
}

/// Whether the daemon knows a provider by the identity: one its
/// configuration's `providers.accepted` lists.
pub fn on_record(daemon: &Daemon, identity: &Identity) -> bool {
    daemon.providers.accepted.contains(identity)
}
