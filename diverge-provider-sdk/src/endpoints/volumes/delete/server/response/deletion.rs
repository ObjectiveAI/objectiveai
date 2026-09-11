//! What a manager says a deletion did.

/// The volume is gone, or it was mounted and is not.
///
/// The two things a provider can answer without failing. A failure is
/// the manager's own error, outside this — and
/// [`Mounted`](Self::Mounted) is deliberately not one, because a
/// caller has to be able to tell "you cannot, and here is why" from
/// "something went wrong", and an error carries no vocabulary for the
/// difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Deletion {
    /// The volume no longer exists.
    Deleted,
    /// The volume is mounted in a running container, and a provider
    /// never deletes one that is. Nothing was changed.
    Mounted,
}
