//! How much a provider relays between the identities connected to it.

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

/// The rate at which a provider relays
/// [`daemons`](crate::provider::endpoints::daemons) messages: at most
/// `messages` sends per sending identity in a fixed window of
/// `seconds`. The rules:
///
/// 1. **Required.** A provider that relays messages states both
///    members; there is no unlimited provider. A provider that relays
///    none states no object at all.
/// 2. **Neither is zero.** Both are at least `1`: an object of either
///    member `0` does not decode.
/// 3. **Per sending identity.** The count is of the sends of one
///    identity, to every receiver together.
///
/// What counts, and when a window begins, is
/// [`daemons::message`](crate::provider::endpoints::daemons::message)'s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MessagesCapabilities {
    /// The most sends one identity makes in one window.
    pub messages: NonZeroU32,
    /// How long one window is, in seconds.
    pub seconds: NonZeroU32,
}
