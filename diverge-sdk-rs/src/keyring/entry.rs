//! What a listing says of an entry.

/// One entry, as [`list`](super::Keyring::list) answers it: its name
/// and when its key was made, never the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The entry's name.
    pub name: String,
    /// When its key was minted, set, or last rotated: RFC 3339, UTC.
    pub made: String,
}
