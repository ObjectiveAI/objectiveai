//! What is done with images.

use serde::{Deserialize, Serialize};

/// The actions over images, each the endpoint of the same name under
/// [`images`](crate::daemon::endpoints::images).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Read an image's DIVERGE.md.
    Info,
}
