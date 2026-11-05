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
    /// The subject the login asks for, as its `login_hint` named it:
    /// what an approval issues tokens to.
    pub login_hint: String,
}

/// A device code a person has approved, as the poll that spends it
/// answers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approved {
    /// The identity approved.
    pub subject: String,
    /// The scope granted.
    pub scope: String,
    /// The audience the access token is for.
    pub audience: String,
    /// When the operator approved it.
    pub auth_time: i64,
}

/// A pending device code, as the operator is shown it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waiting {
    /// The user code, as a person types it: eight characters, no
    /// separator.
    pub user_code: String,
    /// The subject it asks for.
    pub subject: String,
    /// When it stops being good.
    pub expires: i64,
}
