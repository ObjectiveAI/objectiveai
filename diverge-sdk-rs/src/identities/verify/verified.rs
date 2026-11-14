//! A token, verified.

/// Who a verified token names, by what key, and until when.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Verified {
    /// Its `iss`.
    pub issuer: String,
    /// Its `sub`.
    pub subject: String,
    /// Its `exp`, in seconds since the Unix epoch.
    pub expires: i64,
    /// The thumbprint of the key it is bound to, its `cnf.jkt`: what
    /// its presenter's key is held to.
    pub thumbprint: [u8; crate::wire::frame::auth::THUMBPRINT_LEN],
}
