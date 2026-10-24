//! One signing key's record.

/// A signing key, as the database holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The key id.
    pub kid: String,
    /// The key, as a PKCS#1 PEM.
    pub pem: String,
    /// When the key was made, in seconds since the Unix epoch.
    pub made: i64,
}

/// A signing key as a listing reports it: never the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    /// The key id.
    pub kid: String,
    /// When the key was made.
    pub made: i64,
    /// When the key was retired, if it was.
    pub retired: Option<i64>,
}
