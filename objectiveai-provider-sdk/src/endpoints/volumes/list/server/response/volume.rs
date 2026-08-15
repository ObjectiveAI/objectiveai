//! One volume a provider offers.

use serde::{Deserialize, Serialize};

/// A directory a caller may watch, under the name a provider gave it.
///
/// [`name`](Self::name) is what to call it and [`path`](Self::path) is
/// where it is. The two are separate because the provider chooses the
/// name: a listing is what a provider is WILLING to expose, under
/// whatever label makes sense to the person reading it, and that need
/// not be the last component of wherever it happens to live.
///
/// # Why a volume rather than a directory
///
/// Because the name is the point. A directory is a thing on a disk; a
/// volume is a thing a provider decided to OFFER, and the offering is
/// what a caller interacts with. It is also what a
/// [`Mount`](crate::endpoints::laboratories::create::client::request::Mount)
/// names, which is where the word already meant this.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Volume {
    /// What to call this volume, and how to ask for it.
    ///
    /// A label, not a basename: nothing derives it from
    /// [`path`](Self::path) and nothing requires the two to agree.
    ///
    /// It is also the HANDLE. A
    /// [`watch`](crate::endpoints::volumes::watch) names a volume by
    /// this and by nothing else, so two volumes in one listing sharing
    /// a name would make one of them unreachable.
    pub name: String,
    /// Where the volume is, as path components.
    ///
    /// Components rather than a joined string — the same meaning of
    /// "path" as everywhere else in this API, and for the same reason:
    /// a path is a sequence, and joining it would invent a separator
    /// that then has to be escaped out of names containing it.
    ///
    /// This is the frame of reference every path in a watch of this
    /// volume is relative to. A
    /// [`filetree`](crate::shared::filetree) path of
    /// `["src", "main.rs"]` means that file inside THIS volume.
    pub path: Vec<String>,
}
