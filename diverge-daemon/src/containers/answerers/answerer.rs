//! What every answer of one run draws on.

use std::sync::Arc;
use std::time::Instant;

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::provider::client::Answerers;
use tokio::sync::{Mutex, watch};

use crate::containers::fuse::Mounts;
use crate::containers::mcp::Served;
use crate::containers::{Caller, Inflight, Key};
use crate::daemon::Daemon;
use crate::judge::Standing;
use crate::store::AccountId;

/// One run's answerer: the eight traits on one value.
pub struct Answerer {
    /// The daemon.
    pub daemon: Arc<Daemon>,
    /// The container the run is.
    pub key: Key,
    /// Who the run is, for the daemon to attest under `_meta` on every
    /// MCP call it sends outward for it.
    pub caller: Caller,
    /// The container as a sender.
    pub sender: Creator,
    /// The account it runs under, if any: who its `/daemon`
    /// connections are served for, for a record's container.
    pub account: Option<AccountId>,
    /// The standing its `/daemon` connections are served under, for
    /// a dependency: its template's grants, fixed for its life. None
    /// for a record's container, whose account is read fresh, and for
    /// a dependency whose template grants nothing.
    pub standing: Option<Arc<Standing>>,
    /// The provider the run is on: where the container's own paths
    /// are served from, for its dependencies.
    pub provider: Identity,
    /// The mounts the run serves.
    pub mounts: Arc<Mounts>,
    /// The tools the run is served.
    pub served: Arc<Mutex<Served>>,
    /// Where a use is noted.
    pub touched: watch::Sender<Instant>,
    /// The MCP exchanges in flight: the agent's own, which a
    /// dependency shares with its agent.
    pub inflight: Arc<Inflight>,
}

/// The answerers of a run: all eight the one [`Answerer`].
pub type Set = Answerers<Answerer, Answerer, Answerer, Answerer, Answerer, Answerer, Answerer, Answerer>;

impl Answerer {
    /// The eight, as the executors take them.
    pub fn set(self: &Arc<Self>) -> Set {
        Answerers {
            oci: Arc::clone(self),
            authorizer: Arc::clone(self),
            dependencies: Arc::clone(self),
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
