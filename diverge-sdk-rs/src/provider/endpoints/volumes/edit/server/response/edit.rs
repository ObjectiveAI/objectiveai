//! What a manager says an edit did.

/// The volume has the size asked for, or one of two reasons it does
/// not.
///
/// The three things a provider can answer without failing. A failure
/// is the manager's own error, outside this — and the two refusals are
/// deliberately not one, because a caller has to be able to tell "there
/// is no room for that" and "that is smaller than what is in it" from
/// "something went wrong", and an error carries no vocabulary for the
/// difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edit {
    /// The volume has the size asked for.
    Edited,
    /// The provider cannot reserve that many bytes. The size is as it
    /// was.
    InsufficientCapacity,
    /// The volume holds more than the size asked for, so it cannot be
    /// shrunk to it. The size is as it was.
    ContentTooLarge,
}
