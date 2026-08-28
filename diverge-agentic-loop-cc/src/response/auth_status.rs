//! The `auth_status` records: authentication, narrated.

use serde::{Deserialize, Serialize};

/// A `type: "auth_status"` record, emitted only when the run was
/// started with `--enable-auth-status`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthStatus {
    /// Whether an authentication flow is in progress.
    #[serde(rename = "isAuthenticating")]
    pub is_authenticating: bool,
    /// The flow's output lines so far.
    pub output: Vec<String>,
    /// What went wrong, when something did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The record's own id.
    pub uuid: String,
    /// The session.
    pub session_id: String,
}
