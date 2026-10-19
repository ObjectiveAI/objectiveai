//! What a manager says a deletion did.

/// The image is gone, or a running container runs it and it stays.
///
/// The two things a provider can answer without failing. A failure is
/// the manager's own error, outside this — and [`InUse`](Self::InUse)
/// is deliberately not one, because a caller has to be able to tell
/// "stop the container and ask again" from "something went wrong",
/// and an error carries no vocabulary for the difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Deletion {
    /// The image is gone, and the room it took is the store's again.
    Deleted,
    /// A running container of the caller runs the image. Nothing
    /// changed.
    InUse,
}
