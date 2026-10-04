//! How the daemon authenticates to a provider it dials.

use serde::{Deserialize, Serialize};

/// The mode the daemon dials a provider in, and what that mode needs.
/// One object whose one member is the mode, by name, as the provider
/// server's `auth` section names its modes:
/// `{"unbrokered":{"authorization":…}}`. One mode today; a brokered
/// mode is a second member when the wire defines it, and nothing moves.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// The two ends already know each other: the daemon presents a
    /// credential the provider was told to expect.
    Unbrokered {
        /// The credential, as the daemon presents it in its
        /// [`Auth::Unbrokered`](crate::wire::frame::auth::Auth::Unbrokered):
        /// a bearer token, an API key, whatever the provider agreed to.
        /// Given on an add or an edit, and answered by nothing.
        authorization: String,
    },
}
