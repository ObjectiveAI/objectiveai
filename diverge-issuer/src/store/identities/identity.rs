//! What a listing says of an identity.

/// One identity, as [`list`](super::Identities::list) answers it: its
/// subject and when it first was approved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// The subject.
    pub subject: String,
    /// When it first was approved, in seconds since the Unix epoch.
    pub made: i64,
}
