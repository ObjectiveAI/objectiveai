//! What is done with images.

use serde::{Deserialize, Serialize};

/// The actions over images, each the endpoint of the same name under
/// [`images`](crate::daemon::endpoints::images). Snake case on the
/// wire: `"build"`, `"push"`, `"get"`, `"list"`. Reading an image is none of
/// them: [`images::read`](crate::daemon::endpoints::images::read) is
/// held over the providers and the daemons it asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Build an image on a provider, from a Containerfile, into the
    /// daemon's Store there, as
    /// [`images::build`](crate::daemon::endpoints::images::build) does.
    Build,
    /// Push an image into the daemon's Store on a provider, as
    /// [`images::push`](crate::daemon::endpoints::images::push) does.
    Push,
    /// Get one image in a Store of the daemon's, as
    /// [`images::get`](crate::daemon::endpoints::images::get) does.
    Get,
    /// List the images in the daemon's Stores, as
    /// [`images::list`](crate::daemon::endpoints::images::list) does;
    /// the list sends what the grant reaches.
    List,
}
