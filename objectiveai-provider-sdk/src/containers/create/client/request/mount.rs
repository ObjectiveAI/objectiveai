//! One directory made visible inside a container.

use serde::{Deserialize, Serialize};

/// A directory from a listing, mounted into the container.
///
/// # Three paths, two sides
///
/// [`path`](Self::path) is inside the container.
/// [`name`](Self::name) and
/// [`relative_path`](Self::relative_path) together say what to put
/// there, and both are outside it.
///
/// The outside pair is a name and an offset rather than a path,
/// because a path is not something a caller is allowed to state. The
/// name is a
/// [`Directory::name`](crate::filesystem::list::server::response::Directory::name)
/// the provider published, and the offset is relative to whatever that
/// name maps to — so a caller can reach a subdirectory of something it
/// was offered, and nothing else.
///
/// That is the same access model as
/// [`watch`](crate::filesystem::watch), and it holds for the same
/// reason: a provider never validates a path, it resolves a name it
/// chose and then descends. A caller cannot escape upward, because
/// there is no component it can write that means "up" — the offset is
/// components, and `..` is a name, not an instruction.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Mount {
    /// Where the directory appears INSIDE the container, as path
    /// components from the container's root.
    ///
    /// Empty means the root itself, which a provider will almost
    /// certainly refuse — the image's own filesystem is there.
    pub path: Vec<String>,
    /// Which offered directory, by the name a listing gave it.
    ///
    /// Outside the container. Names come from
    /// [`Directory::name`](crate::filesystem::list::server::response::Directory::name)
    /// and mean nothing outside the provider that published them.
    pub name: String,
    /// How far into that directory to start, as path components
    /// relative to it.
    ///
    /// Outside the container. Empty mounts the directory itself, which
    /// is the common case; anything else mounts a subdirectory of it.
    pub relative_path: Vec<String>,
}
