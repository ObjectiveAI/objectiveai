//! What is done to volumes that exist.

use serde::{Deserialize, Serialize};

/// The actions over volumes that exist, which a grant reaches as far as
/// its `within` says. Snake case on the wire: `"get"`, `"list"`,
/// `"delete"`, `"edit"`, `"stat"`, `"download"`, `"upload"`,
/// `"transfer"`, `"filetree"`, `"mount"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// Get one, as
    /// [`volumes::get`](crate::daemon::endpoints::volumes::get) does.
    Get,
    /// List them, as
    /// [`volumes::list`](crate::daemon::endpoints::volumes::list) does;
    /// the list sends what the grant reaches.
    List,
    /// Delete one, as
    /// [`volumes::delete`](crate::daemon::endpoints::volumes::delete)
    /// does.
    Delete,
    /// Change one's size or mode, as
    /// [`volumes::edit`](crate::daemon::endpoints::volumes::edit) does.
    Edit,
    /// Walk one, as
    /// [`volumes::stat`](crate::daemon::endpoints::volumes::stat) does.
    Stat,
    /// Send the client files out of one, as
    /// [`volumes::download`](crate::daemon::endpoints::volumes::download)
    /// does.
    Download,
    /// Put files into one, as
    /// [`volumes::upload`](crate::daemon::endpoints::volumes::upload)
    /// does, and as a transfer landing in one does.
    Upload,
    /// Copy files out of one, as
    /// [`volumes::transfer`](crate::daemon::endpoints::volumes::transfer)
    /// does; where they land is judged by its own grant.
    Transfer,
    /// Name one in a container's mounts — a volume mount of the
    /// provider it is pinned to, or a FUSE mount of a file or a
    /// directory in it — at the container's
    /// [create](crate::daemon::endpoints::agents::create) or
    /// [edit](crate::daemon::endpoints::agents::edit), beside the grant
    /// over the container.
    Mount,
    /// See one's tree, as
    /// [`volumes::filetree`](crate::daemon::endpoints::volumes::filetree)
    /// does.
    Filetree,
}
