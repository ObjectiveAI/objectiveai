//! What may be done with a volume of one mode, on one provider.

use serde::{Deserialize, Serialize};

/// What a provider does with a volume of one
/// [`Mode`](crate::provider::endpoints::volumes::Mode): whether a run
/// may mount it, whether a FUSE serve may hold it, and how many of
/// each, and both, at once.
///
/// The mode itself says the MOST that may be done with a volume in
/// it — any number of running containers but one FUSE serve, never
/// both, for a persistent volume; any number of each, beside one
/// another, for an ephemeral or a read-only one — and this says how
/// much of that the provider does. Each member is a boolean, absent being `false`, and
/// the later ones extend the earlier: `mount_many` means nothing
/// without `mount`, `fuse_serve_many` nothing without `fuse_serve`,
/// `mount_and_fuse_serve` nothing without both; a provider states
/// them consistently, and a reader reads each as the plain fact it
/// is. A request past one is answered by the endpoint's own variant,
/// named below.
///
/// # What a mode allows, at most
///
/// | mode | `mount_many` | `fuse_serve_many` | `mount_and_fuse_serve` |
/// |------|--------------|-------------------|------------------------|
/// | persistent | may | never | never |
/// | ephemeral | may | may | may |
/// | read only | may | may | may |
///
/// A provider holds out no more than the mode allows: a persistent
/// volume's object has `fuse_serve_many` and `mount_and_fuse_serve`
/// `false`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
pub struct VolumeCapabilities {
    /// A run may mount a volume of the mode — a
    /// [`VolumeMount`](crate::shared::containers::request::VolumeMount)
    /// naming one is honored. Absent — `false` — a run naming one is
    /// answered
    /// [`VolumeMountUnsupported`](crate::shared::containers::response::VolumeMountUnsupported).
    #[serde(default)]
    pub mount: bool,
    /// A [`fuse_serve`](crate::provider::endpoints::volumes::fuse_serve)
    /// may hold a volume of the mode. Absent — `false` — one asking
    /// is answered `Unsupported`.
    #[serde(default)]
    pub fuse_serve: bool,
    /// More than one running container may mount a volume of the
    /// mode at once. Absent — `false` — a run naming one that a
    /// running container has is answered
    /// [`VolumeMounted`](crate::shared::containers::response::VolumeMounted).
    #[serde(default)]
    pub mount_many: bool,
    /// More than one FUSE serve may hold a volume of the mode at
    /// once. Absent — `false` — a FUSE serve of one that another FUSE
    /// serve holds is answered `FuseServed`.
    #[serde(default)]
    pub fuse_serve_many: bool,
    /// A running container and a FUSE serve may hold a volume of the
    /// mode at once. Absent — `false` — a run naming one that a FUSE
    /// serve holds is answered
    /// [`VolumeFuseServed`](crate::shared::containers::response::VolumeFuseServed),
    /// and a FUSE serve of one that a running container has is
    /// answered `Mounted`.
    #[serde(default)]
    pub mount_and_fuse_serve: bool,
}
