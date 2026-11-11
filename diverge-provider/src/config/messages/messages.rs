//! The rate messages are relayed at.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

/// The provider relays direct messages between the identities
/// connected to it, at most `messages` sends per sending identity in a
/// fixed window of `seconds`. The rules:
///
/// 1. **Both required.** A section that names messages names both
///    members: there is no unlimited provider.
/// 2. **Neither zero.** Both are at least `1`; a `0` is refused when
///    the file is read.
/// 3. **Held out as is.** The two are the provider's
///    [`MessagesCapabilities`](diverge_sdk::provider::endpoints::info::server::response::MessagesCapabilities),
///    answered by `info` and enforced by every send.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Messages {
    /// The most sends one identity makes in one window.
    pub messages: NonZeroU32,
    /// How long one window is, in seconds.
    pub seconds: NonZeroU32,
}
