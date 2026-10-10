//! The checker.

use std::path::PathBuf;

use diverge_sdk::provider::endpoints::images::check::server::response::{Available, Response, Unavailable};
use crate::protocol::image_checker;
use futures_util::future;

use super::Error;
use diverge_sdk::config::provider::containers::Registry;
use diverge_sdk::shared::containers::request::Image;
use crate::host::container_deployer::name_ok;
use crate::host::tools::podman;

/// The provider's image checker: the store, then the referenced
/// registries the configuration lists, asked.
#[derive(Debug)]
pub struct ImageChecker {
    /// The hosts of the configured registries, in the configuration's
    /// order: those a reference may name.
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

    /// Every reference's name checked as a repository path. In the
    /// store, by digest alone, is available, and nothing else is
    /// asked. Else every referenced registry the configuration lists
    /// is asked at once, as `<registry>/<name>@<digest>`, and a
    /// reference naming a registry not listed is ignored: any yes is
    /// available, every no — and no registry to ask — is unavailable,
    /// and a podman that could not be started, with no yes beside it,
    /// is the failure to answer.
    async fn check(&self, _client_identity: &str, image: &Image) -> Result<Response, Error> {
        if let Some(reference) = image.references.iter().find(|reference| !name_ok(&reference.name)) {
            return Err(Error::Name(reference.name.clone()));
        }
        if podman::image_by_digest(&image.digest).await.map_err(Error::Podman)?.is_some() {
            return Ok(Response::Available(Available::default()));
        }
        let listed = image
            .references
            .iter()
            .filter(|reference| self.registries.contains(&reference.registry));
        let answers = future::join_all(listed.map(|reference| {
            let reference = format!("{}/{}@{}", reference.registry, reference.name, image.digest);
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
