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
    /// The Ed25519 public key its `cnf` names: what its presenter
    /// proves it holds.
    pub key: [u8; crate::wire::frame::auth::KEY_LEN],
}
