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
/// under the exclusive hold, and never a mount's or a FUSE serve's to
/// change. Each STATES the mode it means the volume to have — a run's
/// every [mount](crate::shared::containers::request::VolumeMount::mode),
/// a [`fuse_serve`](super::fuse_serve)'s request — and a volume in another mode
/// refuses it,
/// [`VolumeMode`](crate::shared::containers::response::VolumeMode), so
/// that every container that mounts the volume and every FUSE serve of it
/// runs under the mode it meant, and the mode does not change under
/// it.
///
/// | mode | a container's writes | a FUSE serve's mutations | at rest ([`write`](mod@super::write)) | who may hold it |
/// |------|----------------------|---------------------|--------------------|-----------------|
/// | [`Persistent`](Self::Persistent) | in the volume when the container ends | in the volume | the file lands | up to any number of running containers at once, or ONE FUSE serve, never both |
/// | [`Ephemeral`](Self::Ephemeral) | discarded with the container; every container starts from the volume as it is | taken into a layer of the FUSE serve's own, discarded at the finish; every FUSE serve starts from the volume as it is | the file lands | up to any number of containers and FUSE serves at once |
/// | [`ReadOnly`](Self::ReadOnly) | refused inside the container | refused, [`ReadOnly`](crate::shared::containers::fuse::ack::Frame::ReadOnly) | the file lands | up to any number of containers and FUSE serves at once |
///
/// # Many containers, or one FUSE serve
///
/// A persistent volume may be mounted in any number of running
/// containers at once — each may name it at several container paths
/// in one run — and every one of them mounts the one volume: a
/// change any of them makes is in the volume for every other, as the
/// volume's filesystem makes it visible, and the order of changes
/// made at once is the filesystem's. It has at most one FUSE serve
/// at a time, and never a FUSE serve beside a running container: a
/// serve is the one writer of the volume's bytes, where a container
/// is one of any number on one filesystem. An ephemeral or a
/// read-only volume may be mounted in any number of running
/// containers and served on any number of scopes at once, beside one
/// another. That is the MOST a mode allows; how much
/// of it a provider does for each mode is its
/// [`info`](crate::provider::endpoints::info)'s capability for the
/// mode — `mount`, `fuse_serve`, `mount_many`, `fuse_serve_many`,
/// `mount_and_fuse_serve` — and a run or a FUSE serve that would be
/// one holder more than the provider allows is refused: the run with
/// [`VolumeMounted`](crate::shared::containers::response::VolumeMounted)
/// or
/// [`VolumeFuseServed`](crate::shared::containers::response::VolumeFuseServed),
/// the FUSE serve with its own
/// [`Mounted`](super::fuse_serve::server::response::Frame::Mounted) or
/// [`FuseServed`](super::fuse_serve::server::response::Frame::FuseServed).
/// Either way, nothing examines, reads, writes, walks, resizes or
/// deletes a volume while anything holds it, and the run or the FUSE
/// serve that would start meanwhile is refused as held.
///
/// # On the wire
///
/// One postcard varint — `0` persistent, `1` ephemeral, `2` read
/// only — wherever a volume frame carries it; the strings
/// `persistent`, `ephemeral` and `read_only` as JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Every change is kept. Discriminant `0`.
    #[default]
    Persistent,
    /// Every change is taken and discarded afterwards: with the
    /// container, or with the FUSE serve. Discriminant `1`.
    Ephemeral,
    /// No change is made. Discriminant `2`.
    ReadOnly,
}
