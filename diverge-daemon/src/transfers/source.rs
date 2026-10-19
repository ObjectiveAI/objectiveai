//! Where a transfer, a download or a filetree reads from.

use diverge_sdk::daemon::reference;

use super::Fail;
use crate::containers::{Opened, files};
use crate::content::Pieces;
use crate::daemon::Daemon;
use crate::volumes::{self, Found};

/// What is read.
#[derive(Clone)]
pub enum Source {
    /// A container, opened for the operation.
    Opened(Opened),
    /// A volume, at rest.
    Volume(reference::Volume),
}

impl Source {
    /// The volume at this end, if one.
    pub fn volume(&self) -> Option<&reference::Volume> {
        match self {
            Source::Volume(volume) => Some(volume),
            _ => None,
        }
    }

    /// What is at `path` in the source.
    pub async fn entry_at(&self, daemon: &Daemon, path: &[String]) -> Result<Found, Fail> {
        match self {
            Source::Opened(opened) => files::entry_at(opened, path).await.map_err(Fail::Error),
            Source::Volume(volume) => volumes::entry_at(daemon, volume, path).await.map_err(Fail::from),
        }
    }

    /// The file at `path` in the source, in pieces.
    pub async fn read(&self, daemon: &Daemon, path: &[String]) -> Result<Pieces, Fail> {
        match self {
            Source::Opened(opened) => files::read(opened, path).await.map_err(Fail::Error),
            Source::Volume(volume) => volumes::read(daemon, volume, path).await.map_err(Fail::from),
        }
    }
}
