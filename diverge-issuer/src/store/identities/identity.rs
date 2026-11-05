//! What a listing says of an identity.

/// One identity, as [`list`](super::Identities::list) answers it: its
/// subject and when it was added, never its password's hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// The subject.
    pub subject: String,
    /// When it was added, in seconds since the Unix epoch.
    pub made: i64,
}
