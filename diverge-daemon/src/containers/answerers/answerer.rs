//! What every answer of one run draws on.

use std::sync::Arc;
use std::time::Instant;

use diverge_sdk::daemon::creator::{self, Creator};
use diverge_sdk::provider::client::Answerers;
use tokio::sync::{Mutex, watch};

use crate::containers::Key;
use crate::containers::fuse::Mounts;
use crate::containers::mcp::Served;
use crate::daemon::Daemon;
use crate::store::AccountId;

/// One run's answerer: the eight traits on one value.
pub struct Answerer {
    /// The daemon.
    pub daemon: Arc<Daemon>,
    /// The container the run is.
    pub key: Key,
    /// The container as a sender.
    pub sender: Creator,
    /// The account it runs under, if any: who its `/daemon`
    /// connections are served for.
    pub account: Option<AccountId>,
    /// The root of the chain of dependencies, by name.
    pub root: Option<String>,
    /// The templates down the chain so far.
    pub chain: Vec<String>,
    /// The deployer agent, as the record names it.
    pub deployer: Option<creator::Agent>,
    /// The mounts the run serves.
    pub mounts: Arc<Mounts>,
    /// The tools the run is served.
    pub served: Arc<Mutex<Served>>,
    /// Where a use is noted.
    pub touched: watch::Sender<Instant>,
}

/// The answerers of a run: all eight the one [`Answerer`].
pub type Set = Answerers<Answerer, Answerer, Answerer, Answerer, Answerer, Answerer, Answerer, Answerer>;

impl Answerer {
    /// The eight, as the executors take them.
    pub fn set(self: &Arc<Self>) -> Set {
        Answerers {
            oci: Arc::clone(self),
            authorizer: Arc::clone(self),
            tools: Arc::clone(self),
            postgres: Arc::clone(self),
            daemon: Arc::clone(self),
            vault: Arc::clone(self),
            mcp: Arc::clone(self),
            fuse: Arc::clone(self),
        }
    }

    /// The container was used.
    pub fn touch(&self) {
        self.touched.send_replace(Instant::now());
    }
}
