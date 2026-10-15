//! The volumes a container names in its mounts, checked: there, and
//! the caller allowed to mount them.

use diverge_sdk::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
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
pub async fn mounts(
    conn: &mut PgConnection,
    standing: &Standing,
    daemon: &Daemon,
    pinned: Option<&Provider>,
    file_mounts: &[FuseMount],
    directory_mounts: &[FuseMount],
) -> Result<Mounted, store::Error> {
    let mut named: Vec<reference::Volume> = Vec::new();
    if let Some(provider) = pinned {
        for mount in &provider.volume_mounts {
            named.push(reference::Volume {
                provider: provider.identity.clone(),
                name: mount.volume_name.clone(),
            });
        }
    }
    for mount in file_mounts.iter().chain(directory_mounts) {
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
            fuse_mounts: !file_mounts.is_empty() || !directory_mounts.is_empty(),
            volume_modes: provider.volume_mounts.iter().map(|mount| mount.volume_mode).collect(),
            image: None,
        };
        let lacked = daemon.live.capabilities(&provider.identity).await.and_then(|info| capable::check(&info, &needs).err());
        if let Some(what) = lacked {
            return Ok(Mounted::Unsupported(what));
        }
    }
    for mount in file_mounts.iter().chain(directory_mounts) {
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
