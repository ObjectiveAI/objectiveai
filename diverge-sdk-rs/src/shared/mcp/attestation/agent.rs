//! The agent on the other side.

use serde::{Deserialize, Serialize};

use crate::daemon::key;
use crate::shared::containers::request::Image;

/// An agent, as the daemon attests it: on the `_meta` of every
/// request it sends outward, and of every chunk it says. Written
/// `{"kind":"agent","image":{…},"template":…,"index":…,"name":…}`.
/// An agent is always run from its image, so the image is always
/// here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "kind", rename = "agent")]
pub struct Agent {
    /// The image the agent was made from.
    pub image: Image,
    /// The agent: its template, its index, and its name when it has
    /// one — flat beside the image.
    #[serde(flatten)]
    pub key: key::Agent,
}
