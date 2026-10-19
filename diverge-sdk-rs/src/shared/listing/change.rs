//! What a listing tells.

/// One thing a listing tells: an item come to be listed, one listed
/// that differs from what was last sent, one that ceased to be
/// listed, or the word that the listing as it stood is sent. A list
/// stream's frames are these, each over the family's own record,
/// beside the two answers that end a scope — forbidden, and a
/// failure — which tell a listing nothing; every list `Frame` turns
/// itself into one by its `change`, and the server side of a list
/// diffs a listing into them.
#[derive(Debug, Clone, PartialEq)]
pub enum Change<T> {
    /// The item is listed now, and was not.
    Added(T),
    /// The item is listed still, and differs from what was last sent.
    Changed(T),
    /// The item is listed no more: as it was last sent.
    Removed(T),
    /// Every item listed when the scope opened has been sent.
    Listed,
}
