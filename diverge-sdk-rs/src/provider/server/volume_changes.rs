//! Word that a caller's volumes changed, for the listings open.

use std::sync::Arc;

use tokio::sync::broadcast;

/// How many changes a listing may fall behind by before it is told
/// so, and reads the manager again instead of the changes it missed.
const CHANGES_BEHIND: usize = 256;

/// A nudge, not the change: a volume of the identity's was created,
/// edited, or deleted, by name. What is true of the volume now is
/// the [`VolumeManager`](super::volume_manager::VolumeManager)'s to
/// say; a listing that hears this reads the manager again and tells
/// the difference.
#[derive(Debug, Clone)]
pub struct Changed {
    /// Whose volume.
    pub identity: Arc<str>,
    /// Which volume.
    pub name: String,
}

/// Every change to any caller's volumes, as the volume endpoints
/// make them, for every
/// [`list`](crate::provider::endpoints::volumes::list) scope open.
///
/// One per provider, shared across every connection's
/// [`handle`](super::handle::handle), as the
/// [`Directory`](super::directory::Directory) is: a listing on one
/// connection hears of a create on another. The create, the edit
/// and the delete handlers say so after the manager has answered
/// that it is done; the list handler subscribes before it reads the
/// manager, so a change between the reading and the hearing is
/// either in what was read or in what is heard — and, since a
/// listing answers a nudge by reading again and comparing by name,
/// a change heard that was already read is nothing. This crate
/// keeps no volume state of its own, which is why the word carries
/// nothing but a name.
#[derive(Debug)]
pub struct VolumeChanges {
    changes: broadcast::Sender<Changed>,
}

impl VolumeChanges {
    pub fn new() -> Self {
        let (changes, _) = broadcast::channel(CHANGES_BEHIND);
        VolumeChanges { changes }
    }

    /// Every change from now on, for a listing: subscribed before the
    /// manager is read.
    pub fn subscribe(&self) -> broadcast::Receiver<Changed> {
        self.changes.subscribe()
    }

    /// The identity's volume by that name was created, edited, or
    /// deleted. With no listing open, nothing.
    pub fn changed(&self, identity: &str, name: &str) {
        let _ = self.changes.send(Changed {
            identity: Arc::from(identity),
            name: name.to_string(),
        });
    }
}

impl Default for VolumeChanges {
    fn default() -> Self {
        VolumeChanges::new()
    }
}
