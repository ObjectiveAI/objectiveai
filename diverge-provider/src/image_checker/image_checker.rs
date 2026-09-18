//! The checker.

use std::collections::HashSet;
use std::path::PathBuf;

use diverge_provider_sdk::endpoints::images::check::server::response::{Available, Response, Unavailable};
use diverge_provider_sdk::server::image_checker;
use futures_util::future;

use super::Error;
use crate::config::containers::{Registry, ServerImage};
use crate::tools::podman;

/// The provider's image checker: the configured `server_images`, as
/// a set of pairs, and the configured registries, asked.
#[derive(Debug)]
pub struct ImageChecker {
    /// Every `(name, digest)` the configuration lists as the
    /// provider's own.
    images: HashSet<(String, String)>,
    /// The hosts of the configured registries, in the configuration's
    /// order.
    registries: Vec<String>,
    /// The auth file the deployer wrote for podman, with the
    /// credential of each registry.
    auth_file: PathBuf,
}

impl ImageChecker {
    /// A checker over the `containers` section's `server_images` and
    /// its `podman.registries`, with the auth file the deployer
    /// writes for podman, which every look into a registry is given.
    pub fn new(images: &[ServerImage], registries: &[Registry], auth_file: PathBuf) -> Self {
        ImageChecker {
            images: images
                .iter()
                .map(|image| (image.name.clone(), image.digest.clone()))
                .collect(),
            registries: registries.iter().map(|registry| registry.host.clone()).collect(),
            auth_file,
        }
    }
}

impl image_checker::ImageChecker for ImageChecker {
    type Error = Error;

    /// Listed is available. Else every registry asked at once: any
    /// yes is available, every no is unavailable, and a podman that
    /// could not be started, with no yes beside it, is the failure to
    /// answer.
    async fn check(&self, _client_identity: &str, name: &str, digest: &str) -> Result<Response, Error> {
        if self.images.contains(&(name.to_string(), digest.to_string())) {
            return Ok(Response::Available(Available::default()));
        }
        let answers = future::join_all(self.registries.iter().map(|host| {
            let reference = format!("{host}/{name}@{digest}");
            async move { podman::manifest_exists(&self.auth_file, &reference).await }
        }))
        .await;
        if answers.iter().any(|answer| matches!(answer, Ok(true))) {
            return Ok(Response::Available(Available::default()));
        }
        match answers.into_iter().find_map(Result::err) {
            Some(error) => Err(Error::Podman(error)),
            None => Ok(Response::Unavailable(Unavailable::default())),
        }
    }
}
