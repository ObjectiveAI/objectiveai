//! A token, verified.

/// Who a verified token names, and until when.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Verified {
    /// Its `iss`.
    pub issuer: String,
    /// Its `sub`.
    pub subject: String,
    /// Its `exp`, in seconds since the Unix epoch.
    pub expires: i64,
}
