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
/// absent — `false`, the empty list, the object of every member
/// `false` — so a provider states only what it does. A member a
/// reader does not know is ignored: see [`Frame`](super::Frame).
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
    /// What may be done with a volume in the
    /// [`Persistent`](Mode::Persistent) mode. Every member `false` —
    /// the object absent — the provider keeps no volume in the mode:
    /// no listed volume is in it, and a
    /// [`create`](crate::provider::endpoints::volumes::create) or an
    /// [`edit`](crate::provider::endpoints::volumes::edit) naming it
    /// is answered `ModeUnsupported`. See
    /// [`kept`](VolumeCapabilities::kept).
    #[serde(default)]
    pub volumes_persistent: VolumeCapabilities,
    /// What may be done with a volume in the
    /// [`Ephemeral`](Mode::Ephemeral) mode. Every member `false`, as
    /// for [`volumes_persistent`](Self::volumes_persistent).
    #[serde(default)]
    pub volumes_ephemeral: VolumeCapabilities,
    /// What may be done with a volume in the
    /// [`ReadOnly`](Mode::ReadOnly) mode. Every member `false`, as
    /// for [`volumes_persistent`](Self::volumes_persistent).
    #[serde(default)]
    pub volumes_read_only: VolumeCapabilities,
}

impl Capabilities {
    /// Whether the provider keeps volumes at all: any mode among the
    /// three. `false` is every volume endpoint answering
    /// `Unsupported`.
    pub fn volumes(&self) -> bool {
        self.volumes_persistent.kept() || self.volumes_ephemeral.kept() || self.volumes_read_only.kept()
    }

    /// What may be done with a volume of `mode` here: `None` for a
    /// mode the provider keeps no volumes in — its object every
    /// member `false`.
    pub fn volume(&self, mode: Mode) -> Option<&VolumeCapabilities> {
        let object = match mode {
            Mode::Persistent => &self.volumes_persistent,
            Mode::Ephemeral => &self.volumes_ephemeral,
            Mode::ReadOnly => &self.volumes_read_only,
        };
        object.kept().then_some(object)
    }
}
