//! The volumes a container names in its mounts, checked: there, and
//! the caller allowed to mount them.

use diverge_sdk::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};
use diverge_sdk::daemon::agent_log::Identity;
use diverge_sdk::daemon::endpoints::daemons::DaemonId;
use diverge_sdk::daemon::grant::daemons::Over as DaemonsOver;
use diverge_sdk::daemon::grant::volumes::Over;
use diverge_sdk::daemon::reference;
use diverge_sdk::provider::endpoints::volumes::list::server::response::Volume;
use sqlx::PgConnection;

use super::Mounted;
use crate::containers::capable;
use crate::daemon::Daemon;
use crate::judge::{self, Standing};
use crate::store::{self, volumes as volume_tags};
use crate::volumes::{self, Listed};

/// Every volume named — in the pinned provider's volume mounts, and
/// by each cross-provider mount — must be one its provider lists, else
/// `Error`; one its provider's capabilities serve as the mount asks —
/// mounted, for the pinned provider's own; FUSE served, for a
/// cross-provider one; and the pinned provider making FUSE mounts
/// where any is named — else `Unsupported`, judged from the cached
/// info of every provider connected now; and reached by a `mount`
/// grant of the standing, else `Forbidden`: naming a volume in a
/// container's mounts takes `mount` over it, beside the grant over
/// the container.
///
/// A mount of a volume on ANOTHER daemon is none of that. It is
/// judged first, and by one thing: the record its `daemon` names must
/// exist, else `Error`, and the standing must hold `relay` over it —
/// the one permission to use a remote daemon — else `Forbidden`. A
/// `route` with no `daemon` is `Error`. Nothing of the other daemon's
/// is looked at: not its providers, not its volumes, not what its
/// providers serve. Whether it serves the volume to this daemon's
/// account is its own to judge, at the run's start.
pub async fn mounts(
    conn: &mut PgConnection,
    standing: &Standing,
    daemon: &Daemon,
    pinned: Option<&Provider>,
    file_mounts: &[FuseMount],
    directory_mounts: &[FuseMount],
) -> Result<Mounted, store::Error> {
    // The mounts of volumes on other daemons, judged by `relay` over
    // each record named, once each.
    let mut records: Vec<DaemonId> = Vec::new();
    for mount in file_mounts.iter().chain(directory_mounts) {
        let Some(id) = mount.daemon else {
            if !mount.route.is_empty() {
                return Ok(Mounted::Error(format!("the mount of the volume {} states a route and no daemon", mount.volume_name)));
            }
            continue;
        };
        if records.contains(&id) {
            continue;
        }
        records.push(id);
        let Some(record) = store::daemons::by_id(conn, id, false).await? else {
            return Ok(Mounted::Error(format!("no remote daemon's record has the ID `{id}`")));
        };
        let reported = record.report(daemon.live.is_daemon_connected(record.id).await);
        if !judge::daemons::over(standing, DaemonsOver::Relay, &reported) {
            return Ok(Mounted::Forbidden);
        }
    }
    // From here, this daemon's own volumes.
    let file_mounts: Vec<&FuseMount> = file_mounts.iter().filter(|mount| mount.daemon.is_none()).collect();
    let directory_mounts: Vec<&FuseMount> = directory_mounts.iter().filter(|mount| mount.daemon.is_none()).collect();
    let mut named: Vec<reference::Volume> = Vec::new();
    if let Some(provider) = pinned {
        for mount in &provider.volume_mounts {
            named.push(reference::Volume {
                provider: provider.identity.clone(),
                name: mount.volume_name.clone(),
            });
        }
    }
    for mount in file_mounts.iter().chain(&directory_mounts) {
        named.push(reference::Volume {
            provider: mount.provider.clone(),
            name: mount.volume_name.clone(),
        });
    }
    if named.is_empty() {
        return Ok(Mounted::Ok);
    }
    if !judge::volumes::holds(standing, Over::Mount) {
        return Ok(Mounted::Forbidden);
    }
    if let Some(provider) = pinned {
        let needs = capable::Needs {
            volume_modes: provider.volume_mounts.iter().map(|mount| mount.volume_mode).collect(),
            image: None,
            ..capable::Needs::default()
        };
        let lacked = daemon.live.capabilities(&provider.identity).await.and_then(|info| capable::check(&info, &needs).err());
        if let Some(what) = lacked {
            return Ok(Mounted::Unsupported(what));
        }
    }
    for mount in file_mounts.iter().chain(&directory_mounts) {
        let lacked = daemon.live.capabilities(&mount.provider).await.and_then(|info| capable::fuse_serve(&info, mount.volume_mode).err());
        if let Some(what) = lacked {
            return Ok(Mounted::Unsupported(format!("the volume {} cannot be served: {what}", mount.volume_name)));
        }
    }
    // One listing per provider named, since a listing is a scope
    // opened and stopped, and a container may name several volumes of
    // one provider.
    let mut listings: Vec<(Identity, Vec<Volume>)> = Vec::new();
    for volume in named {
        if !listings.iter().any(|(provider, _)| *provider == volume.provider) {
            match volumes::list(daemon, &volume.provider).await {
                Ok(listed) => listings.push((volume.provider.clone(), listed)),
                Err(error) => return Ok(Mounted::Error(format!("the volume {} could not be looked up: {error}", volume.name))),
            }
        }
        let found = listings
            .iter()
            .find(|(provider, _)| *provider == volume.provider)
            .and_then(|(_, listed)| listed.iter().find(|listed| listed.name == volume.name))
            .cloned();
        let Some(found) = found else {
            return Ok(Mounted::Error(format!("the provider lists no volume named {}", volume.name)));
        };
        let (agents, tools) = volumes::mounters(conn, &volume).await?;
        let tags = volume_tags::of_volume(conn, &volume).await?;
        let listed = Listed {
            provider: volume.provider.clone(),
            volume: found,
            agents,
            tools,
            tags,
        };
        if !judge::volumes::over(standing, Over::Mount, &listed) {
            return Ok(Mounted::Forbidden);
        }
    }
    Ok(Mounted::Ok)
}
