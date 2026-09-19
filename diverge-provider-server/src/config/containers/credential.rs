//! What the provider presents to a registry.

use serde::{Deserialize, Serialize};

/// A username and a password, the pair every registry's token
/// endpoint takes: a personal access token is a password here, and
/// the username beside it is whatever the registry requires with one.
///
/// Both are required: a registry that takes one alone is not one the
/// provider logs in to. A registry that takes neither is listed with
/// no credential at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Credential {
    /// The username.
    pub username: String,
    /// The password, or the token standing as one.
    pub password: String,
}
