//! Why a mutation did not happen, owned.

/// A mutation the caller did not make: the owned form of the two
/// refusing kinds of an [`Ack`](super::Frame), which a caller's
/// server answers with and the answering side turns into the frame.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Refused {
    /// The volume behind the mount is read only — its mode is
    /// [`ReadOnly`](crate::endpoints::volumes::Mode::ReadOnly) — so
    /// no change is made there, and none is faked. The program sees a
    /// read-only filesystem.
    ReadOnly,
    /// It did not happen, and this says why, for a reader rather
    /// than a program.
    Error(String),
}
