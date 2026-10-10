//! The checker.

use std::path::PathBuf;

use diverge_sdk::provider::endpoints::images::check::server::response::{Available, Response, Unavailable};
use crate::protocol::image_checker;
use futures_util::future;

use super::Error;
use diverge_sdk::config::provider::containers::Registry;
use crate::host::tools::podman;

/// The provider's image checker: the store, then the configured
/// registries, asked.
#[derive(Debug)]
pub struct ImageChecker {
    /// The hosts of the configured registries, in the configuration's
    /// order.
    registries: Vec<String>,
    /// The auth file the deployer wrote for podman, with the
    /// credential of each registry.
    auth_file: PathBuf,
}

impl ImageChecker {
    /// A checker over the `containers` section's `podman.registries`,
    /// with the auth file the deployer writes for podman, which every
    /// look into a registry is given.
    pub fn new(registries: &[Registry], auth_file: PathBuf) -> Self {
        ImageChecker {
            registries: registries.iter().map(|registry| registry.host.clone()).collect(),
            auth_file,
        }
    }
}

impl image_checker::ImageChecker for ImageChecker {
    type Error = Error;

    /// In the store, by digest alone, is available, and nothing else
    /// is asked. Else every registry asked at once: any yes is
    /// available, every no is unavailable, and a podman that could
    /// not be started, with no yes beside it, is the failure to
    /// answer.
    async fn check(&self, _client_identity: &str, name: &str, digest: &str) -> Result<Response, Error> {
        if podman::image_by_digest(digest).await.map_err(Error::Podman)?.is_some() {
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
