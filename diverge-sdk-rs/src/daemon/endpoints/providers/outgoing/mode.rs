//! How the daemon authenticates to a provider it dials.

use serde::{Deserialize, Serialize};

/// The mode the daemon dials a provider in, and what that mode needs.
/// JSON-tagged by `kind`: one variant today, `unbrokered`, and a tag
/// all the same, for the reason the wire's
/// [`Auth`](crate::wire::frame::auth::Auth) ships a mode byte for one
/// mode — a brokered mode is coming, and a tag added later is a wire
/// break.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
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
