//! The three modes a volume is in: persistent, ephemeral, or read
//! only.

use serde::{Deserialize, Serialize};

/// What becomes of a change to a volume — kept, discarded, or never
/// made — and so who may hold the volume at once.
///
/// A fact of the volume: stated by a
/// [`create`](super::create::client::request::Frame::mode), reported
/// by a [`listing`](super::list::server::response::Volume::mode),
/// changed only by an [`edit`](super::edit::client::request::Change::Mode)
/// under the exclusive hold, and never a mount's or a serve's to
/// choose. Every container that mounts the volume and every
/// [`serve`](super::serve) of it is bound under the mode the volume
/// has when it takes the volume, and the mode does not change under
/// it.
///
/// | mode | a container's writes | a serve's mutations | at rest ([`write`](mod@super::write)) | who may hold it |
/// |------|----------------------|---------------------|--------------------|-----------------|
/// | [`Persistent`](Self::Persistent) | in the volume when the container ends | in the volume | the file lands | ONE user: one running container, or one serve |
/// | [`Ephemeral`](Self::Ephemeral) | discarded with the container; every container starts from the volume as it is | taken into a layer of the serve's own, discarded at the finish; every serve starts from the volume as it is | the file lands | any number of containers and serves at once |
/// | [`ReadOnly`](Self::ReadOnly) | refused inside the container | refused, [`ReadOnly`](crate::shared::containers::fuse::ack::Frame::ReadOnly) | the file lands | any number of containers and serves at once |
///
/// # One user, or many
///
/// A persistent volume has exactly one user at a time: one running
/// container — which may name it at several container paths in one
/// run — or one serve. A run or a serve that would be a second user
/// is refused, the run with
/// [`VolumeHeld`](crate::shared::containers::response::VolumeHeld)
/// and the serve with the
/// [`mounted`](super::refusal::mounted) refusal. An ephemeral or a
/// read-only volume may be mounted in any number of running
/// containers and served on any number of scopes at once. Either
/// way, nothing examines, reads, writes, walks, resizes or deletes a
/// volume while anything holds it.
///
/// # On the wire
///
/// One postcard varint — `0` persistent, `1` ephemeral, `2` read
/// only — wherever a volume frame carries it; the strings
/// `persistent`, `ephemeral` and `read_only` as JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Every change is kept. Discriminant `0`.
    #[default]
    Persistent,
    /// Every change is taken and discarded afterwards: with the
    /// container, or with the serve. Discriminant `1`.
    Ephemeral,
    /// No change is made. Discriminant `2`.
    ReadOnly,
}
