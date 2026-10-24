//! The checker.

use diverge_sdk::shared::Identity;
use std::path::PathBuf;
use std::sync::Arc;

use diverge_sdk::provider::endpoints::images::check::server::response::{Available, Response, Unavailable};
use crate::protocol::image_checker;
use futures_util::future;

use super::Error;
use crate::config::containers::Registry;
use diverge_sdk::provider::endpoints::images::check::client::request::Sources;
use diverge_sdk::shared::containers::request::Image;
use crate::host::container_deployer::name_ok;
use crate::host::image_manager::Store;
use crate::host::tools::podman;

/// The provider's image checker: the caller's own images, then the
/// referenced registries the configuration lists, asked; never the
/// cache.
#[derive(Debug)]
pub struct ImageChecker {
    /// The client image store, where the caller's own images are;
    /// `None` where the configuration has none.
    store: Option<Arc<Store>>,
    /// The hosts of the configured registries, in the configuration's
    /// order: those a reference may name.
    registries: Vec<String>,
    /// The auth file the deployer wrote for podman, with the
    /// credential of each registry.
    auth_file: PathBuf,
}

impl ImageChecker {
    /// A checker over the client image store, if there is one, and
    /// the `containers` section's `podman.registries`, with the auth
    /// file the deployer writes for podman, which every look into a
    /// registry is given.
    pub fn new(store: Option<Arc<Store>>, registries: &[Registry], auth_file: PathBuf) -> Self {
        ImageChecker {
            store,
            registries: registries.iter().map(|registry| registry.host.clone()).collect(),
            auth_file,
        }
    }
}

impl image_checker::ImageChecker for ImageChecker {
    type Error = Error;

    /// Looked where `sources` says, local first. `client` is the
    /// caller's own images in the client image store, by digest, and
    /// never another identity's; with no store, never available.
    /// Having the image ends it. `registries`, asked only when the
    /// local source asked did not have it: every reference's name
    /// checked as a repository path, then
    /// every referenced registry the configuration lists asked at
    /// once, as `<registry>/<name>@<digest>`, a reference naming a
    /// registry not listed ignored. Any yes is available; every no —
    /// and no source with the image — is unavailable; a podman that
    /// could not be started, with no yes beside it, is the failure to
    /// answer.
    async fn check(&self, client_identity: &Identity, image: &Image, sources: &Sources) -> Result<Response, Error> {
        let named = image.references.iter().find(|reference| !name_ok(&reference.name));
        if let (true, Some(reference)) = (sources.registries, named) {
            return Err(Error::Name(reference.name.clone()));
        }
        if let (true, Some(store)) = (sources.client, &self.store) {
            if store.find_digest(client_identity, &image.digest).await.is_some() {
                return Ok(Response::Available(Available::default()));
            }
        }
        if !sources.registries {
            return Ok(Response::Unavailable(Unavailable::default()));
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
