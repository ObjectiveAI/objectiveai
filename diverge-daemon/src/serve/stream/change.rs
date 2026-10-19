//! What a listing tells.

/// One thing a listing tells its client: an item come to be listed,
/// one listed that differs from what was last sent, one that ceased
/// to be listed, or the word that the listing as it stood is sent.
#[derive(Debug, Clone, PartialEq)]
pub enum Change<I> {
    /// The item is listed now, and was not.
    Added(I),
    /// The item is listed still, and differs from what was last sent.
    Changed(I),
    /// The item is listed no more: as it was last sent.
    Removed(I),
    /// Every item listed when the scope opened has been sent.
    Listed,
}
