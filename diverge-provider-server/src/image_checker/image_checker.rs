//! The checker.

use dashmap::DashSet;
use diverge_provider_sdk::endpoints::images::check::server::response::{Available, Response, Unavailable};
use diverge_provider_sdk::server::image_checker;
use futures_util::future;

use super::Error;
use crate::config::containers::Podman;
use crate::tools::podman;

/// The provider's image checker: podman's store, asked under every
/// name the image might be held by.
#[derive(Debug)]
pub struct ImageChecker {
    /// The configured registries' hosts, in the configuration's
    /// order: the prefixes the store may hold a name under.
    hosts: Vec<String>,
    /// Every `(name, digest)` the store has answered yes for.
    known: DashSet<(String, String)>,
}

impl ImageChecker {
    /// A checker over the `podman` section's registries.
    pub fn new(podman: &Podman) -> Self {
        ImageChecker {
            hosts: podman.registries.iter().map(|registry| registry.host.clone()).collect(),
            known: DashSet::new(),
        }
    }

    /// Every reference the store might hold `name` under: the bare
    /// name, then the name under each configured host, in the
    /// configuration's order.
    fn candidates(&self, name: &str) -> Vec<String> {
        std::iter::once(name.to_string())
            .chain(self.hosts.iter().map(|host| format!("{host}/{name}")))
            .collect()
    }

    /// One reference asked of the store, by digest.
    async fn held(&self, candidate: &str, digest: &str) -> Result<bool, Error> {
        Ok(podman::image_exists(&format!("{candidate}@{digest}")).await?)
    }
}

impl image_checker::ImageChecker for ImageChecker {
    type Error = Error;

    /// Every candidate asked at once. Any yes is available, and
    /// remembered; every no is unavailable; a store that could not be
    /// asked, with no yes beside it, is the failure to answer.
    async fn check(&self, _client_identity: &str, name: &str, digest: &str) -> Result<Response, Error> {
        let key = (name.to_string(), digest.to_string());
        if self.known.contains(&key) {
            return Ok(Response::Available(Available::default()));
        }
        let candidates = self.candidates(name);
        let answers = future::join_all(candidates.iter().map(|candidate| self.held(candidate, digest))).await;
        if answers.iter().any(|answer| matches!(answer, Ok(true))) {
            self.known.insert(key);
            return Ok(Response::Available(Available::default()));
        }
        match answers.into_iter().find_map(Result::err) {
            Some(error) => Err(error),
            None => Ok(Response::Unavailable(Unavailable::default())),
        }
    }
}
