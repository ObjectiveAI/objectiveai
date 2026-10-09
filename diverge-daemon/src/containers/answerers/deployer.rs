//! The container's declared dependencies, deployed.

use std::sync::Arc;

use diverge_sdk::provider::client::ToolDeployer;
use diverge_sdk::shared::containers::tools::Tool;
use diverge_sdk::shared::error::Error;

use super::Answerer;
use crate::containers::deploy::{self, Deploy};

/// Every dependency answered as [`deploy`](crate::containers::deploy)
/// states, before the container's id is out; the first unmet is the
/// run's error. Asked on an agent's run alone: a tool container
/// declares no dependencies, and its run scope carries no such ask.
impl ToolDeployer for Answerer {
    async fn deploy(&self, tools: Vec<Tool>) -> Result<(), Error> {
        let deploy = Deploy {
            daemon: Arc::clone(&self.daemon),
            user: self.key,
            sender: self.sender.clone(),
            root: self.root.clone(),
            deployer: self.deployer.clone(),
            served: Arc::clone(&self.served),
        };
        deploy::deploy(&deploy, tools).await
    }
}
