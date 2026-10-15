//! What a provider does, of everything this specification lets a
//! provider do.

use serde::{Deserialize, Serialize};

use super::VolumeCapabilities;
use crate::provider::endpoints::volumes::Mode;

/// Which of the specification's optional parts this provider serves.
///
/// Every provider serves every endpoint's wire — it reads every
/// request and answers it — but not every provider does everything a
/// request may ask: one has no image store, one pulls from no
/// registry, one has no FUSE on its host, one keeps volumes of some
/// modes only, and keeps them more narrowly than the mode allows.
/// Each such part is one member here, and a request past it is not
/// a failure but a refusal the endpoint has a variant of its own for,
/// named below member by member. A caller that read these composes
/// no such request; one that did not reads the refusal.
///
/// # Flat, and tolerant
///
/// One object, every member at the top, each a boolean, a list or an
/// object of booleans; no member nests a capability inside another
/// but the three volume modes, whose object says what may be done
/// with a volume of the mode. A member absent is the capability
/// absent — `false`, the empty list, no object — so a provider states
/// only what it does. A member a reader does not know is ignored:
/// see [`Frame`](super::Frame).
///
/// # No mode is no volumes
///
/// A provider with none of the three volume modes keeps no volumes,
/// and every endpoint of
/// [`volumes`](crate::provider::endpoints::volumes) — the eleven —
/// answers `Unsupported`: nothing is listed, nothing is made, and
/// there is no volume for a run or a build to name. [`volumes`](Self::volumes)
/// is that test.
///
/// # What is not here
///
/// The volume endpoints themselves, which every provider that keeps
/// volumes serves;
/// [`daemons`](crate::provider::endpoints::daemons), which is the
/// provider's configuration and refused as such; and every quantity
/// — how many containers, how much disk — which is a limit, not a
/// capability, and answered where it binds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Capabilities {
    /// The provider keeps an image store for its callers: it answers
    /// [`images::list`](crate::provider::endpoints::images::list),
    /// [`stat`](crate::provider::endpoints::images::stat),
    /// [`build`](crate::provider::endpoints::images::build),
    /// [`delete`](crate::provider::endpoints::images::delete) and
    /// [`oci`](crate::provider::endpoints::images::oci), and an
    /// [`images::check`](crate::provider::endpoints::images::check)
    /// asking the `client` source looks in it. Absent — `false` —
    /// each of the five answers `Unsupported`, and a check asking only
    /// `client` answers `NoSource`.
    #[serde(default)]
    pub images_store: bool,
    /// The registries the provider pulls from: each a host as a
    /// [`Reference`](crate::shared::containers::request::Reference)
    /// names it — `docker.io`, `ghcr.io`, `registry.example.com:5000`
    /// — and the whole of them. An
    /// [`images::check`](crate::provider::endpoints::images::check)
    /// asking the `registries` source consults exactly these, and a
    /// reference naming any other is ignored; a run's image whose
    /// references name none of these, and that no store the provider
    /// has holds, cannot be supplied. Absent — empty — the provider
    /// pulls from no registry, and a check asking only `registries`
    /// answers `NoSource`.
    #[serde(default)]
    pub images_registries: Vec<String>,
    /// The provider mounts a run's FUSE mounts — the
    /// [`FuseMount`](crate::shared::containers::request::FuseMount)s
    /// of its request — served by the caller. Absent — `false` — a
    /// run naming any is answered `FuseMountsUnsupported`.
    #[serde(default)]
    pub containers_fuse_mount: bool,
    /// The provider FUSE serves a running container's subtree:
    /// [`containers::fuse_serve`](crate::provider::endpoints::containers::fuse_serve)
    /// is answered. Absent — `false` — it answers `Unsupported`.
    #[serde(default)]
    pub containers_fuse_serve: bool,
    /// The provider keeps volumes in the
    /// [`Persistent`](Mode::Persistent) mode, and this is what may be
    /// done with one. Absent, no listed volume is in the mode, and a
    /// [`create`](crate::provider::endpoints::volumes::create) or an
    /// [`edit`](crate::provider::endpoints::volumes::edit) naming it
    /// is answered `ModeUnsupported`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_persistent: Option<VolumeCapabilities>,
    /// The provider keeps volumes in the
    /// [`Ephemeral`](Mode::Ephemeral) mode, and this is what may be
    /// done with one. Absent, as for
    /// [`volume_persistent`](Self::volume_persistent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_ephemeral: Option<VolumeCapabilities>,
    /// The provider keeps volumes in the
    /// [`ReadOnly`](Mode::ReadOnly) mode, and this is what may be
    /// done with one. Absent, as for
    /// [`volume_persistent`](Self::volume_persistent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_read_only: Option<VolumeCapabilities>,
}

impl Capabilities {
    /// Whether the provider keeps volumes at all: any mode among the
    /// three. `false` is every volume endpoint answering
    /// `Unsupported`.
    pub fn volumes(&self) -> bool {
        self.volume_persistent.is_some() || self.volume_ephemeral.is_some() || self.volume_read_only.is_some()
    }

    /// What may be done with a volume of `mode` here: `None` for a
    /// mode the provider keeps no volumes in.
    pub fn volume(&self, mode: Mode) -> Option<&VolumeCapabilities> {
        match mode {
            Mode::Persistent => self.volume_persistent.as_ref(),
            Mode::Ephemeral => self.volume_ephemeral.as_ref(),
            Mode::ReadOnly => self.volume_read_only.as_ref(),
        }
    }
}
