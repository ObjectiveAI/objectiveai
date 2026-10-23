//! How the link stands, as the page is told.

use serde::{Deserialize, Serialize};

/// How a [`Link`](super::link::Link) stands: not yet dialled, being
/// dialled, up, or down with the reason. Serialized as
/// `{"state": "idle"}`, `{"state": "dialling"}`,
/// `{"state": "connected"}` or
/// `{"state": "failed", "message": "..."}`, which is what the
/// TypeScript SDK's `wire.proxy.WireStatus` reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum WireStatus {
    /// Nothing has used the link since it was made or reset.
    Idle,
    /// A use is dialling now.
    Dialling,
    /// The connection is up: the last dial succeeded and nothing has
    /// seen it end since.
    Connected,
    /// The last dial failed, or the connection ended; every send is
    /// refused until the link is reset.
    Failed {
        /// Why, in words.
        message: String,
    },
}
