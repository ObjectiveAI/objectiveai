//! A volume made, changed, dropped, or measured, on its provider.

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::endpoints::volumes::stat::server::response::Stat;
use diverge_sdk::daemon::reference;
use diverge_sdk::provider::endpoints::volumes::Mode;
use diverge_sdk::provider::endpoints::volumes::create::client::{execute as create_execute, request as create_request};
use diverge_sdk::provider::endpoints::volumes::delete::client::{execute as delete_execute, request as delete_request};
use diverge_sdk::provider::endpoints::volumes::edit::client::request::Change;
use diverge_sdk::provider::endpoints::volumes::edit::client::{execute as edit_execute, request as edit_request};
use diverge_sdk::provider::endpoints::volumes::stat::client::{execute as stat_execute, request as stat_request};

use super::{Fail, provider};
use crate::daemon::Daemon;

/// What a create came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Made {
    /// The volume exists now.
    Created,
    /// The provider has not the capacity.
    InsufficientCapacity,
}

/// What an edit came to; a volume mounted or otherwise held is the
/// `Fail`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edited {
    /// The volume is as asked.
    Edited,
    /// The provider has not the capacity for the new size.
    InsufficientCapacity,
    /// The content is larger than the new size.
    ContentTooLarge,
}

/// What a delete came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dropped {
    /// The volume is gone.
    Deleted,
    /// A container of the provider's mounts it.
    Mounted,
}

/// A volume made on the provider.
pub async fn create(daemon: &Daemon, identity: &Identity, name: String, bytes: u64, mode: Mode) -> Result<Made, Fail> {
    let handle = provider::handle(daemon, identity).await?;
    match create_execute::execute(&handle, &create_request::Frame { name, bytes, mode }).await {
        Ok(()) => Ok(Made::Created),
        Err(create_execute::ExecuteError::InsufficientCapacity) => Ok(Made::InsufficientCapacity),
        Err(create_execute::ExecuteError::Provider(error)) => Err(Fail::refusal(&error)),
        Err(error) => Err(Fail::failed(&error)),
    }
}

/// The volume's size, mode, or both changed.
pub async fn edit(daemon: &Daemon, volume: &reference::Volume, change: Change) -> Result<Edited, Fail> {
    let handle = provider::handle(daemon, &volume.provider).await?;
    let request = edit_request::Frame {
        name: volume.name.clone(),
        change,
    };
    match edit_execute::execute(&handle, &request).await {
        Ok(()) => Ok(Edited::Edited),
        Err(edit_execute::ExecuteError::InsufficientCapacity) => Ok(Edited::InsufficientCapacity),
        Err(edit_execute::ExecuteError::ContentTooLarge) => Ok(Edited::ContentTooLarge),
        Err(edit_execute::ExecuteError::Provider(error)) => Err(Fail::refusal(&error)),
        Err(error) => Err(Fail::failed(&error)),
    }
}

/// The volume dropped from its provider.
pub async fn delete(daemon: &Daemon, volume: &reference::Volume) -> Result<Dropped, Fail> {
    let handle = provider::handle(daemon, &volume.provider).await?;
    let request = delete_request::Frame {
        name: volume.name.clone(),
    };
    match delete_execute::execute(&handle, &request).await {
        Ok(()) => Ok(Dropped::Deleted),
        Err(delete_execute::ExecuteError::Mounted) => Ok(Dropped::Mounted),
        Err(delete_execute::ExecuteError::Provider(error)) => Err(Fail::refusal(&error)),
        Err(error) => Err(Fail::failed(&error)),
    }
}

/// The volume measured: the bytes used and the `dirhash` of its
/// content.
pub async fn stat(daemon: &Daemon, volume: &reference::Volume) -> Result<Stat, Fail> {
    let handle = provider::handle(daemon, &volume.provider).await?;
    let request = stat_request::Frame {
        name: volume.name.clone(),
    };
    match stat_execute::execute(&handle, &request).await {
        Ok(stat) => Ok(Stat {
            bytes_used: stat.bytes_used,
            dirhash: stat.dirhash,
        }),
        Err(stat_execute::ExecuteError::Provider(error)) => Err(Fail::refusal(&error)),
        Err(error) => Err(Fail::failed(&error)),
    }
}
