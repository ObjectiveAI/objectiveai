//! The checker.

use std::collections::HashSet;

use diverge_provider_sdk::endpoints::images::check::server::response::{Available, Response, Unavailable};
use diverge_provider_sdk::server::image_checker;

use super::Error;
use crate::config::containers::Containers;

/// The provider's image checker: the configured `server_images`, as
/// a set of the pairs a caller may name.
#[derive(Debug)]
pub struct ImageChecker {
    /// Every `(name, digest)` the configuration lists.
    images: HashSet<(String, String)>,
}

impl ImageChecker {
    /// A checker over the `containers` section's `server_images`.
    pub fn new(containers: &Containers) -> Self {
        ImageChecker {
            images: containers
                .server_images
                .iter()
                .map(|image| (image.name.clone(), image.digest.clone()))
                .collect(),
        }
    }

    /// Whether the pair is listed: what a `server` deploy asks too,
    /// so a check and a run never disagree.
    pub fn offers(&self, name: &str, digest: &str) -> bool {
        self.images.contains(&(name.to_string(), digest.to_string()))
    }
}

impl image_checker::ImageChecker for ImageChecker {
    type Error = Error;

    /// Listed is available; anything else is unavailable.
    async fn check(&self, _client_identity: &str, name: &str, digest: &str) -> Result<Response, Error> {
        Ok(if self.offers(name, digest) {
            Response::Available(Available::default())
        } else {
            Response::Unavailable(Unavailable::default())
        })
    }
}
