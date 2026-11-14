//! What narrows a list of images.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::agent_log::Identity;

/// The filter over the images in the daemon's Stores: every member
/// optional, and every one given a condition an image passes or does
/// not. In a [list request](super::Frame) it is flattened into the
/// request and narrows what the daemon sends. It is not what an images
/// grant reaches by: a grant reaches providers, see
/// [`grant::images`](crate::daemon::grant::images). A filter with no
/// member given passes every image.
///
/// # Any one of
///
/// A member that lists candidates matches an image that is any one of
/// them, or is held by any one of them. An empty list is absent, and
/// matches every image.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Filter {
    /// In the daemon's Store on any one of these providers: see
    /// [`Identity`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub providers: Vec<Identity>,
    /// Any one of these digests, `<algorithm>:<hex>`, on whichever
    /// provider.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub digests: Vec<String>,
    /// The earliest `created` to list, inclusive; absent, no earliest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_from: Option<DateTime<Utc>>,
    /// The latest `created` to list, inclusive; absent, no latest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_to: Option<DateTime<Utc>>,
}
