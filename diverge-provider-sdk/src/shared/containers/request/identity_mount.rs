//! One thing the caller mounts into the container.

use serde::{Deserialize, Serialize};

/// Content the caller wants present in the container's filesystem:
/// where, and what.
///
/// One shape for a file and for a directory — which it is, the field
/// it sits in says ([`file_mounts`](super::Container::file_mounts) or
/// [`directory_mounts`](super::Container::directory_mounts)), and the
/// [`identity`](Self::identity) grammar agrees. The server MUST mount every
/// one — read-only — before the container starts: the request
/// naming it IS the requirement. What the server does not hold it
/// MAY fetch from the client, by the hash, over the fetch exchanges.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdentityMount {
    /// Where it appears inside the container, as path components
    /// from the container's root — the shape every path in this
    /// crate takes, as
    /// [`container_path`](super::VolumeMount::container_path)
    /// does for a volume.
    ///
    /// No component is empty, `.` or `..` — an offset is components,
    /// and `..` is a name, not an instruction — and no mount's path
    /// is a prefix of another mount's, file or directory. Mounting
    /// INTO a directory the image owns is the point; mounts stacking
    /// on each other is not. Empty would name the root, which a
    /// provider refuses: the image's own filesystem is there.
    pub container_path: Vec<String>,
    /// The content's size-bearing identity:
    /// `<size>:<base64url sha256>` — the size in bytes, then the
    /// hash. For a file the hash is of its bytes; for a directory it
    /// is of its manifest — one `<hash> <size> <path>` line per file,
    /// paths relative and `/`-separated, sorted bytewise — and the
    /// size is the total. Which of the two it is, the field it sits in
    /// says, as does the fetch that asks for it; the value does not
    /// need to.
    ///
    /// Because the size rides the identity, a server can refuse an
    /// oversized request up front, as a request error, with nothing
    /// fetched.
    pub identity: String,
}
