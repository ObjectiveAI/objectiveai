//! The agent inside the harness.

use serde::{Deserialize, Serialize};

use crate::daemon::key;
use crate::shared::containers::request::Image;

/// An agent of this daemon's, as the daemon attests it: the `agent`
/// kind of a [`Caller`](super::Caller), on the `_meta` of every
/// request its container sends outward, and what every chunk it says
/// is stamped with. Its image and its key, flat beside the kind:
/// `"image":{…},"template":…,"index":…,"name":…`. An agent is always
/// run from its image, so the image is always here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Agent {
    /// The image the agent was made from.
    pub image: Image,
    /// The agent: its template, its index, and its name when it has
    /// one — flat beside the image.
    #[serde(flatten)]
    pub key: key::Agent,
}
