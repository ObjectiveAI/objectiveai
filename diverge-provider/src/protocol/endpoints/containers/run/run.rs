//! What every task of a run shares.

use diverge_sdk::shared::Identity;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use tokio::sync::{Mutex, Notify};
use tokio::task::JoinSet;

use super::begin::Begin;
use super::ledger::Ledger;
use super::pairs::Pairs;
use super::watched::Watched;
use diverge_sdk::wire::client::handle::Handle;
use diverge_sdk::container_proxy::outside::endpoints::filesystem::tree::client::execute as tree;
use crate::protocol::directory::Directory;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;

/// One container being served: the scope it was asked for on, the
/// connection to its proxy, and everything the tasks serving it
/// share.
///
/// A run handler makes one after the id is out. Every task the machinery
/// spawns takes an [`Arc`] of it, and [`shutdown`](Self::shutdown) is
/// how they all end.
pub(crate) struct Run {
    /// The scope, on which every channel is opened or answered.
    pub scope: Arc<ScopeHandle>,
    /// Whose scope this is: the identity of the caller that opened it,
    /// which a transfer's rule is checked against.
    pub identity: Arc<Identity>,
    /// Every container the provider is running, for a transfer to
    /// find the other one and to ask who may reach it.
    pub directory: Arc<Directory>,
    /// The one connection to the container's proxy, on which tree,
    /// read and write scopes are opened — and a transfer's read, its
    /// write going to another container's.
    pub proxy: Handle,
    /// The begin scope on it: where the proxy's asks arrive and where
    /// the family's own exchanges go.
    pub begin: Begin,
    /// Every mount the provider watches itself, for a filetree to
    /// open beside the proxy's tree and merge in.
    pub watched: Arc<[Watched]>,
    /// Every mount the container holds, for a mount asked for while
    /// it runs to be refused as overlapping or under a taken id; a
    /// FUSE mount is let go when its relay ends. What the proxy's tree
    /// leaves out is read off it when a tree scope is opened — see
    /// [`ignored`](Self::ignored) — so the list is never kept twice
    /// and never outlives a mount.
    pub mounts: Mutex<Ledger>,
    /// Database connections whose caller half has not opened.
    /// The daemon connections announced to the caller and not yet
    /// taken. Its own, apart from [`pairs`](Self::pairs): an id
    /// announced as one kind must not resolve a half of the other, and
    /// a registry per kind is what makes that so rather than hoped.
    pub daemons: Pairs,
    pub pairs: Pairs,
    /// The container is gone: the proxy's asks ended.
    pub over: Notify,
    /// Every tree scope open on the proxy for this run, by its scope
    /// number: the one kind of scope this end opens that does not end
    /// by itself, stopped by [`shutdown`](Self::shutdown) before the
    /// task serving it is aborted.
    pub trees: Mutex<HashMap<u32, tree::ExecuteHandle>>,
    /// Every terminal the caller holds open on this run, by id: each
    /// one's shell killed by [`shutdown`](Self::shutdown) as the trees
    /// are stopped.
    pub terminals: super::serve::terminal::Terminals,
    tasks: Mutex<JoinSet<()>>,
}

impl Run {
    pub(crate) fn new(
        scope: Arc<ScopeHandle>,
        identity: Arc<Identity>,
        directory: Arc<Directory>,
        proxy: Handle,
        begin: Begin,
        watched: Arc<[Watched]>,
        mounts: Ledger,
    ) -> Self {
        Run {
            scope,
            identity,
            directory,
            proxy,
            begin,
            watched,
            mounts: Mutex::new(mounts),
            daemons: Pairs::new(),
            pairs: Pairs::new(),
            over: Notify::new(),
            trees: Mutex::new(HashMap::new()),
            terminals: Default::default(),
            tasks: Mutex::new(JoinSet::new()),
        }
    }

    /// One more task of this run, and every task already done reaped
    /// first, under the same lock: so the set holds the tasks still
    /// running, however many the run has spawned over its life.
    pub(crate) async fn spawn(&self, task: impl Future<Output = ()> + Send + 'static) {
        let mut tasks = self.tasks.lock().await;
        while tasks.try_join_next().is_some() {}
        tasks.spawn(task);
    }

    /// Every path the proxy's tree leaves out, as of now: every FUSE
    /// mount's the container holds — the request's, and each made or
    /// being made since — since a FUSE mount is the caller's own
    /// answers and a tree over it would report them back to the
    /// caller; and every mount's of a volume the provider watches
    /// itself. A tree already open was opened with the list as it was.
    pub(crate) async fn ignored(&self) -> Vec<Vec<String>> {
        let mut ignored = self.mounts.lock().await.fuse_paths();
        ignored.extend(self.watched.iter().map(|watched| watched.container_path.clone()));
        ignored
    }

    /// End every task: stop every tree scope still open on the
    /// proxy, then abort the tasks and wait for the aborts.
    ///
    /// Aborted rather than drained, because a task waiting on the
    /// caller — a fate that never comes, a channel nobody finishes —
    /// would otherwise hold the finish hostage. The trees are stopped
    /// here, on the way out, rather than by the tasks on a signal: a
    /// task woken and then aborted before it runs would never have
    /// sent the stop, and would leave its trees watching a container
    /// that goes on.
    pub(crate) async fn shutdown(&self) {
        for (_, handle) in self.trees.lock().await.drain() {
            let _ = handle.stop().await;
        }
        self.terminals.stop_all().await;
        let mut tasks = self.tasks.lock().await;
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
    }

    /// One answer on a channel the caller opened, when it encoded.
    pub(crate) async fn respond(&self, channel: u32, payload: Option<Vec<u8>>) {
        if let Some(payload) = payload {
            self.scope.send_channel_response(channel, &payload).await;
        }
    }

    /// The end of a channel the caller opened.
    pub(crate) async fn finish(&self, channel: u32) {
        self.scope.send_channel_response_finish(channel).await;
    }
}

/// One response on the scope's main stream, when it encoded.
pub(crate) async fn send(scope: &ScopeHandle, payload: Option<Vec<u8>>) {
    if let Some(payload) = payload {
        scope.send_response(&payload).await;
    }
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Run")
            .field("scope", &self.scope)
            .field("begin", &self.begin)
            .field("daemons", &self.daemons)
            .field("pairs", &self.pairs)
            .finish()
    }
}
