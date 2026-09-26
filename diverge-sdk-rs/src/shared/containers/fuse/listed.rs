//! One entry of a listed directory, owned.

use super::Kind;

/// One entry of a listed directory, owned: what a `readdir` shows,
/// as a caller's server or a provider's volume answers a
/// [`list`](super::list) before it is an [`Entry`](super::Entry) on
/// the wire.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Listed {
    /// The entry's name, one path component.
    pub name: String,
    /// File or directory.
    pub kind: Kind,
}
