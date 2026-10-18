//! What every answer of one run draws on.

use std::sync::Arc;
use std::time::Instant;

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::provider::client::Answerers;
use indexmap::IndexMap;
use tokio::sync::{Mutex, watch};

use crate::containers::fuse::Mounts;
use crate::containers::mcp::Served;
use crate::containers::secrets::Secrets;
use diverge_sdk::shared::mcp::attestation;

use crate::containers::{Inflight, Key};
use crate::daemon::Daemon;
use crate::judge::Dependent;
use crate::store::{AccountId, DependencyToolId};

/// One run's answerer: the seven traits on one value.
pub struct Answerer {
    /// The daemon.
    pub daemon: Arc<Daemon>,
    /// The container the run is.
    pub key: Key,
    /// The agent the run is, as the daemon attests it under `_meta` on
    /// every MCP call it sends outward for it: a caller of the agent
    /// kind. None for a tool's run, which serves no tool and calls
    /// nothing outward.
    pub caller: Option<attestation::Caller>,
    /// The container as a sender.
    pub sender: Creator,
    /// The account it runs under, if any: who its `/daemon`
    /// connections are served for, for a tool record's container.
    /// None for an agent, which holds no account and asks the daemon
    /// nothing, and for a dependency.
    pub account: Option<AccountId>,
    /// What its `/daemon` connections are served under, for a
    /// dependency: its template's grants, fixed for its life, and
    /// whose `self` it is. None for a record's container, whose
    /// account is read fresh, and for a dependency whose template
    /// grants nothing.
    pub dependent: Option<Dependent>,
    /// The provider the run is on: where the container's own paths
    /// are served from, for its dependencies.
    pub provider: Identity,
    /// The mounts the run serves.
    pub mounts: Arc<Mounts>,
    /// The tools the run is served.
    pub served: Arc<Mutex<Served>>,
    /// Where a use is noted.
    pub touched: watch::Sender<Instant>,
    /// The MCP exchanges in flight: the agent's own, which a transient
    /// dependency shares with its agent.
    pub inflight: Arc<Inflight>,
    /// The services an agent's registration declared, once its
    /// dependencies were deployed: `None` until then, and still
    /// `None` for a program that declared nothing, which is never
    /// asked about. What the start settles the agent's attachments
    /// by.
    pub services: Mutex<Option<Vec<DependencyToolId>>>,
    /// The names the container's template declares: what its
    /// registration may take.
    pub declared: Vec<String>,
    /// Each name the template declares to the vault key it is bound
    /// to: the record's own binding — a dependency's, the binding of
    /// the agent that spawns it, as of this start.
    pub binding: IndexMap<String, String>,
    /// The run's secrets, once its registration took them: `None`
    /// until then, and still `None` for a program that registered
    /// nothing. Stashed here since no run exists at registration; the
    /// start moves it into the run, or gives back what it holds when
    /// the provider refuses the run after all.
    pub secrets: Mutex<Option<Secrets>>,
}

/// The answerers of a run: all six the one [`Answerer`].
pub type Set = Answerers<Answerer, Answerer, Answerer, Answerer, Answerer, Answerer>;

impl Answerer {
    /// The six, as the executors take them.
    pub fn set(self: &Arc<Self>) -> Set {
        Answerers {
            oci: Arc::clone(self),
            registrar: Arc::clone(self),
            postgres: Arc::clone(self),
            daemon: Arc::clone(self),
            mcp: Arc::clone(self),
            fuse: Arc::clone(self),
        }
    }

    /// The container was used.
    pub fn touch(&self) {
        self.touched.send_replace(Instant::now());
    }
}
