//! One directory made visible inside a container.

use serde::{Deserialize, Serialize};

/// A directory from a listing, mounted into the container.
///
/// Host side first, then the container side — source before
/// destination, the order a mount reads in everywhere else.
///
/// # Why the host side is a name and an offset
///
/// Because a host path is not something a caller is allowed to state.
/// [`host_name`](Self::host_name) is a
/// [`Directory::name`](crate::filesystem::list::server::response::Directory::name)
/// the provider published, and
/// [`host_relative_path`](Self::host_relative_path) descends from
/// wherever that maps to — so a caller reaches a subdirectory of
/// something it was offered, and nothing else.
///
/// That is the same access model as
/// [`watch`](crate::filesystem::watch), and it holds for the same
/// reason: a provider never validates a path, it resolves a name it
/// chose and then descends. A caller cannot escape upward, because
/// there is no component it can write that means "up" — the offset is
/// components, and `..` is a name, not an instruction.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Mount {
    /// Which offered directory, by the name a listing gave it.
    ///
    /// Names come from
    /// [`Directory::name`](crate::filesystem::list::server::response::Directory::name)
    /// and mean nothing outside the provider that published them.
    pub host_name: String,
    /// How far into that directory to start, as path components
    /// relative to it.
    ///
    /// Empty mounts the directory itself, which is the common case;
    /// anything else mounts a subdirectory of it.
    pub host_relative_path: Vec<String>,
    /// Where it appears inside the container, as path components from
    /// the container's root.
    ///
    /// Empty means the root itself, which a provider will almost
    /// certainly refuse — the image's own filesystem is there.
    pub container_path: Vec<String>,
}
