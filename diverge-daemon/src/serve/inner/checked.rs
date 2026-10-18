//! What a check of a create's member came to.

/// The member checked, or why it was not taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Checked<T> {
    /// The member, as the record keeps it.
    Ok(T),
    /// The account named is none the daemon has.
    NoAccount,
    /// The caller holds no grant that reaches what was named.
    Forbidden,
    /// What was named is not there, in a sentence: the create's
    /// error.
    Error(String),
}
