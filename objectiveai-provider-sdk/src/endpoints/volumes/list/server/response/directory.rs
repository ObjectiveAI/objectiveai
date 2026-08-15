//! One directory a provider offers.

use serde::{Deserialize, Serialize};

/// A directory a caller may watch.
///
/// [`name`](Self::name) is what to call it and [`path`](Self::path) is
/// where it is. The two are separate because the provider chooses the
/// name: a listing is what a provider is WILLING to expose, under
/// whatever label makes sense to the person reading it, and that need
/// not be the last component of wherever it happens to live.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Directory {
    /// What to call this directory, and how to ask for it.
    ///
    /// A label, not a basename: nothing derives it from
    /// [`path`](Self::path) and nothing requires the two to agree.
    ///
    /// It is also the HANDLE. A
    /// [`watch`](crate::endpoints::volumes::watch) names a directory by this
    /// and by nothing else, so two directories in one listing sharing
    /// a name would make one of them unreachable.
    pub name: String,
    /// Where the directory is, as path components.
    ///
    /// Components rather than a joined string — the same meaning of
    /// "path" as everywhere else in this API, and for the same reason:
    /// a path is a sequence, and joining it would invent a separator
    /// that then has to be escaped out of names containing it.
    ///
    /// This is the value a caller names to watch it, and it is the
    /// frame of reference every path in that watch is relative to. A
    /// [`filetree`](crate::shared::filetree) path of `["src", "main.rs"]` means
    /// that file inside THIS directory.
    pub path: Vec<String>,
}
