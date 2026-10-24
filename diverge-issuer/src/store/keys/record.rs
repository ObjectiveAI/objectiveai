//! One signing key's record.

/// A signing key, as the database names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The key id.
    pub kid: String,
    /// The keyring entry the key's PEM is kept in.
    pub entry: String,
    /// When the key was made, in seconds since the Unix epoch.
    pub made: i64,
}
