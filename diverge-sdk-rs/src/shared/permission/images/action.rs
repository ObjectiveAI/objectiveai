//! What is done with images.

use serde::{Deserialize, Serialize};

/// The actions over images, each the endpoint of the same name under
/// [`images`](crate::daemon::endpoints::images). Reading an image is
/// none of them: [`images::read`](crate::daemon::endpoints::images::read)
/// is held over the providers and the daemons it asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Build an image on a provider, from a Containerfile.
    Build,
}
