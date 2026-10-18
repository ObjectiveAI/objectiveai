//! What a manager says a creation did.

/// The volume exists, or the provider had no room for it.
///
/// The two things a provider can answer without failing. A failure is
/// the manager's own error, outside this — and
/// [`InsufficientCapacity`](Self::InsufficientCapacity) is deliberately
/// not one, because a caller has to be able to tell "there is no room
/// for that" from "something went wrong", and an error carries no
/// vocabulary for the difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Creation {
    /// The volume exists, at the size asked for.
    Created,
    /// The provider cannot reserve that many bytes. Nothing exists.
    InsufficientCapacity,
}
