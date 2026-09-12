//! One volume made visible inside a container.

use serde::{Deserialize, Serialize};

/// A volume from a listing, mounted into the container.
///
/// Host side first, then the container side — source before
/// destination, the order a mount reads in everywhere else.
///
/// # Why the host side is a name and an offset
///
/// Because a host path is not something a caller is allowed to state.
/// [`host_name`](Self::host_name) is a
/// [`Volume::name`](crate::endpoints::volumes::list::server::response::Volume::name)
/// the provider published, and
/// [`host_relative_path`](Self::host_relative_path) descends from
/// wherever that maps to — so a caller reaches a subdirectory of
/// something it was offered, and nothing else.
///
/// That is the same access model as
/// [`watch`](crate::endpoints::volumes::watch), and it holds for the same
/// reason: a provider never validates a path, it resolves a name it
/// chose and then descends. A caller cannot escape upward, because
/// there is no component it can write that means "up" — the offset is
/// components, and `..` is a name, not an instruction.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct VolumeMount {
    /// Which offered volume, by the name a listing gave it.
    ///
    /// Names come from
    /// [`Volume::name`](crate::endpoints::volumes::list::server::response::Volume::name)
    /// and mean nothing outside the provider that published them.
    pub host_name: String,
    /// How far into that volume to start, as path components
    /// relative to it.
    ///
    /// Empty mounts the volume itself, which is the common case;
    /// anything else mounts a subdirectory of it.
    pub host_relative_path: Vec<String>,
    /// Where it appears inside the container, as path components from
    /// the container's root.
    ///
    /// Empty means the root itself, which a provider will almost
    /// certainly refuse — the image's own filesystem is there.
    pub container_path: Vec<String>,
    /// Whether the container's changes to the volume outlive the
    /// container.
    ///
    /// `true`: every change the container makes is in the volume
    /// when the container ends. `false`: the volume is as it was
    /// before the run when the container ends. Either way the
    /// container sees the volume's content as of its start. How a
    /// provider makes `false` hold — an overlay, a copy — is its own.
    ///
    /// Present, always: a request states it and a provider never
    /// infers it.
    pub persist: bool,
}
