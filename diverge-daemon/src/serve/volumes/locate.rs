//! A volume found, as a handler needs it: on record, listed by its
//! provider, with what mounts it.

use diverge_sdk::daemon::reference;
use sqlx::PgConnection;

use crate::daemon::Daemon;
use crate::serve::inner;
use crate::store::{self, volumes as volume_tags};
use crate::volumes::{self, Listed};

/// What locating a volume came to.
#[derive(Debug, Clone, PartialEq)]
pub enum Located {
    /// The volume, as a list reports it.
    Volume(Listed),
    /// The provider is not on record, or lists no such volume.
    None,
    /// The provider is on record but could not be asked: not
    /// connected, or it failed.
    Failed(String),
}

/// The volume: `None` for a provider not on record or a name its
/// provider does not list, `Failed` for a provider that could not be
/// asked, else the listing's entry with the agents and tools that
/// mount it and the tags on it.
pub async fn locate(conn: &mut PgConnection, daemon: &Daemon, volume: &reference::Volume) -> Result<Located, store::Error> {
    if !inner::on_record(conn, &volume.provider).await? {
        return Ok(Located::None);
    }
    let found = match volumes::find(daemon, volume).await {
        Ok(Some(found)) => found,
        Ok(None) | Err(volumes::Fail::NotFound) => return Ok(Located::None),
        Err(fail) => return Ok(Located::Failed(fail.to_string())),
    };
    let (agents, tools) = volumes::mounters(conn, volume).await?;
    let tags = volume_tags::of_volume(conn, volume).await?;
    Ok(Located::Volume(Listed {
        provider: volume.provider.clone(),
        volume: found,
        agents,
        tools,
        tags,
    }))
}
