//! What narrows a list of providers.

use serde::{Deserialize, Serialize};

use crate::shared::Identity;

/// The filter over providers: every member optional, and every one
/// given a condition a provider passes or does not. In a [list
/// request](super::Frame) it is flattened into the request and narrows
/// what the daemon sends. A filter with no member given passes every
/// provider.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Filter {
    /// Any one of these identities. Absent when empty, and then any.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identities: Vec<Identity>,
    /// Whether the daemon's [`Status`](crate::daemon::endpoints::providers::Status)
    /// with it is `connected`: `true`, or not — disconnected, or a
    /// different version — `false`; absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connected: Option<bool>,
}
