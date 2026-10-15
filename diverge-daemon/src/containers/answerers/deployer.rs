//! The container's declared dependencies, deployed.

use diverge_sdk::provider::client::DependencyDeployer;
use diverge_sdk::shared::containers::dependencies::Template;
use diverge_sdk::shared::error::Error;

use super::Answerer;
use crate::containers::deploy;

/// Every dependency deployed as [`deploy`](crate::containers::deploy)
/// states, before the container's id is out — the id told here so the
/// agent's paths can be served into them; the first that cannot be
/// is the run's error. Asked on an agent's run alone: a tool
/// container declares no dependencies, and its run scope carries no
/// such ask.
impl DependencyDeployer for Answerer {
    async fn deploy(&self, id: String, dependencies: Vec<Template>) -> Result<(), Error> {
        deploy::deploy(self, id, dependencies).await
    }
}
