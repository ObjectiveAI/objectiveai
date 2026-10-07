//! The volumes a container names in its mounts, checked: there, and
//! the caller allowed to mount them.

use diverge_sdk::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};
use diverge_sdk::daemon::grant::volumes::Over;
use diverge_sdk::daemon::reference;
use sqlx::PgConnection;

use super::Checked;
use crate::daemon::Daemon;
use crate::judge::{self, Standing};
use crate::store;
use crate::volumes::{self, Listed};

/// Every volume named — in the pinned provider's volume mounts, and
/// by each cross-provider mount — must be one its provider lists, else
/// `Error`, and reached by a `mount` grant of the standing, else
/// `Forbidden`: naming a volume in a container's mounts takes `mount`
/// over it, beside the grant over the container.
pub async fn mounts(
    conn: &mut PgConnection,
    standing: &Standing,
    daemon: &Daemon,
    pinned: Option<&Provider>,
    file_mounts: &[FuseMount],
    directory_mounts: &[FuseMount],
) -> Result<Checked<()>, store::Error> {
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
        return Ok(Checked::Ok(()));
    }
    if !judge::volumes::holds(standing, Over::Mount) {
        return Ok(Checked::Forbidden);
    }
    for volume in named {
        let found = match volumes::find(daemon, &volume).await {
            Ok(Some(found)) => found,
            Ok(None) => return Ok(Checked::Error(format!("the provider lists no volume named {}", volume.name))),
            Err(error) => return Ok(Checked::Error(format!("the volume {} could not be looked up: {error}", volume.name))),
        };
        let (agents, tools) = volumes::mounters(conn, &volume).await?;
        let listed = Listed {
            provider: volume.provider.clone(),
            volume: found,
            agents,
            tools,
        };
        if !judge::volumes::over(standing, Over::Mount, &listed) {
            return Ok(Checked::Forbidden);
        }
    }
    Ok(Checked::Ok(()))
}
