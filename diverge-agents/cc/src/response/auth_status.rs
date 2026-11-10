//! The `auth_status` records: authentication, narrated.

use serde::Deserialize;

/// A `type: "auth_status"` record, emitted only when the run was
/// started with `--enable-auth-status`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AuthStatus {
    /// Always `auth_status`.
    pub r#type: AuthStatusType,
    /// Whether an authentication flow is in progress.
    #[serde(rename = "isAuthenticating")]
    pub is_authenticating: bool,
    /// The flow's output lines so far.
    pub output: Vec<String>,
    /// What went wrong, when something did.
    pub error: Option<String>,
    /// The record's own id.
    pub uuid: String,
    /// The session.
    pub session_id: String,
}

/// The `auth_status` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum AuthStatusType {
    /// The only value.
    #[default]
    AuthStatus,
}

impl AuthStatus {
    /// Whether authentication FAILED — [`error`](Self::error)
    /// carried; progress narration says nothing.
    pub fn failed(&self) -> bool {
        self.error.is_some()
    }

    /// The failure as a notification's (or an HTTP error's) message
    /// body.
    pub fn message(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": "auth",
            "error": self.error,
            "output": self.output,
        })
    }
}
