//! One device code.

/// A device code as it is issued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct New {
    /// The SHA-256 of the device code, as lowercase hex.
    pub hash: String,
    /// The user code, as a person types it: eight characters, no
    /// separator.
    pub user_code: String,
    /// The scope asked for.
    pub scope: String,
    /// The audience the access token will be for.
    pub audience: String,
    /// When it stops being good.
    pub expires: i64,
    /// The seconds the app waits between polls.
    pub interval: i64,
}

/// A device code a person has approved, as the poll that spends it
/// answers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approved {
    /// The identity that signed in.
    pub subject: String,
    /// The scope granted.
    pub scope: String,
    /// The audience the access token is for.
    pub audience: String,
    /// When the identity signed in.
    pub auth_time: i64,
}

/// A pending device code, as the device page shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// The scope it asks for.
    pub scope: String,
}
