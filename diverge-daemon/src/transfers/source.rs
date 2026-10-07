//! Where a transfer, a download or a filetree reads from.

use std::path::PathBuf;

use diverge_sdk::daemon::endpoints::resources::Kind;
use diverge_sdk::daemon::reference;
use diverge_sdk::shared::filetree::response::Node;

use super::Fail;
use crate::containers::{Opened, files};
use crate::content::{self, Pieces};
use crate::daemon::Daemon;
use crate::volumes::{self, Found};

/// What is read.
#[derive(Clone)]
pub enum Source {
    /// A container, opened for the operation.
    Opened(Opened),
    /// A volume, at rest.
    Volume(reference::Volume),
    /// A resource the daemon holds: its bytes on disk, and which
    /// kind.
    Resource {
        /// The file, or the directory's root.
        root: PathBuf,
        /// Which kind.
        kind: Kind,
    },
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
            Source::Resource { root, kind } => {
                if *kind == Kind::File {
                    return Ok(if path.is_empty() {
                        Found::File(Node::File {
                            name: "file".to_string(),
                            size: None,
                            created_at: None,
                            modified_at: None,
                        })
                    } else {
                        Found::Missing
                    });
                }
                if content::inside(path).is_err() {
                    return Ok(Found::Missing);
                }
                Ok(match content::at(root, path).await {
                    Some(content::Found::File(at)) => Found::File(Node::File {
                        name: at.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default(),
                        size: None,
                        created_at: None,
                        modified_at: None,
                    }),
                    Some(content::Found::Directory(dir)) => {
                        Found::Directory(content::tree(&dir).await.map_err(|error| Fail::Error(error.to_string()))?)
                    }
                    None => Found::Missing,
                })
            }
        }
    }

    /// The file at `path` in the source, in pieces.
    pub async fn read(&self, daemon: &Daemon, path: &[String]) -> Result<Pieces, Fail> {
        match self {
            Source::Opened(opened) => files::read(opened, path).await.map_err(Fail::Error),
            Source::Volume(volume) => volumes::read(daemon, volume, path).await.map_err(Fail::from),
            Source::Resource { root, kind } => {
                let at = if *kind == Kind::File { root.clone() } else { content::join(root, path) };
                Ok(content::from_file(at))
            }
        }
    }
}
