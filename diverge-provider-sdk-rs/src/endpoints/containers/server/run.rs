//! What every task of a run shares.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use tokio::sync::{Mutex, Notify};
use tokio::task::JoinSet;

use super::begin::Begin;
use super::pairs::Pairs;
use super::watched::Watched;
use crate::client::handle::Handle;
use crate::container_proxy_endpoints::filesystem::tree::client::execute as tree;
use crate::server::directory::Directory;
use crate::server::scope_handle::ScopeHandle;

/// One container being served: the scope it was asked for on, the
/// connection to its proxy, and everything the tasks serving it
/// share.
///
/// A run handler makes one after the id is out; a connect handler
/// makes one from what the directory holds. Every task the machinery
/// spawns takes an [`Arc`] of it, and [`shutdown`](Self::shutdown) is
/// how they all end.
pub(crate) struct Run {
    /// The scope, on which every channel is opened or answered.
    pub scope: Arc<ScopeHandle>,
    /// Whose scope this is: the identity of the caller that opened it,
    /// which a transfer's rule is checked against.
    pub identity: Arc<str>,
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
    /// Every path the proxy's tree leaves out: every FUSE mount's,
    /// and every mount's of a volume the provider watches itself.
    pub ignore: Vec<Vec<String>>,
    /// Every mount the provider watches itself, for a filetree to
    /// open beside the proxy's tree and merge in.
    pub watched: Arc<[Watched]>,
    /// Database connections whose caller half has not opened.
    pub pairs: Pairs,
    /// The container is gone: the proxy's asks ended, or the run a
    /// connector joined is over.
    pub over: Notify,
    /// Every tree scope open on the proxy for this run, by its scope
    /// number: the one kind of scope this end opens that does not end
    /// by itself, stopped by [`shutdown`](Self::shutdown) before the
    /// task serving it is aborted.
    pub trees: Mutex<HashMap<u32, tree::ExecuteHandle>>,
    tasks: Mutex<JoinSet<()>>,
}

impl Run {
    pub(crate) fn new(
        scope: Arc<ScopeHandle>,
        identity: Arc<str>,
        directory: Arc<Directory>,
        proxy: Handle,
        begin: Begin,
        ignore: Vec<Vec<String>>,
        watched: Arc<[Watched]>,
    ) -> Self {
        Run {
            scope,
            identity,
            directory,
            proxy,
            begin,
            ignore,
            watched,
            pairs: Pairs::new(),
            over: Notify::new(),
            trees: Mutex::new(HashMap::new()),
            tasks: Mutex::new(JoinSet::new()),
        }
    }

    /// One more task of this run.
    pub(crate) async fn spawn(&self, task: impl Future<Output = ()> + Send + 'static) {
        self.tasks.lock().await.spawn(task);
    }

    /// End every task: stop every tree scope still open on the
    /// proxy, then abort the tasks and wait for the aborts.
    ///
    /// Aborted rather than drained, because a task waiting on the
    /// caller — a fate that never comes, a channel nobody finishes —
    /// would otherwise hold the finish hostage. The trees are stopped
    /// here, on the way out, rather than by the tasks on a signal: a
    /// task woken and then aborted before it runs would never have
    /// sent the stop, and a connector that leaves would leave its
    /// trees watching a container that goes on.
    pub(crate) async fn shutdown(&self) {
        for (_, handle) in self.trees.lock().await.drain() {
            let _ = handle.stop().await;
        }
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
            .field("pairs", &self.pairs)
            .finish()
    }
}
