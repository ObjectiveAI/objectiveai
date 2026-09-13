//! What every task of a run shares.

use std::future::Future;
use std::sync::Arc;

use tokio::sync::{Mutex, Notify};
use tokio::task::JoinSet;

use super::begin::Begin;
use super::pairs::Pairs;
use crate::client::handle::Handle;
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
    /// The one connection to the container's proxy, on which tree,
    /// read and write scopes are opened.
    pub proxy: Handle,
    /// The begin scope on it: where the proxy's asks arrive and where
    /// the family's own exchanges go.
    pub begin: Begin,
    /// Every mount's path, which a filetree leaves out.
    pub ignore: Vec<Vec<String>>,
    /// Database connections whose caller half has not opened.
    pub pairs: Pairs,
    /// The container is gone: the proxy's asks ended, or the run a
    /// connector joined is over.
    pub over: Notify,
    /// The run is ending: a task holding a scope on the proxy that
    /// does not end by itself — a tree — stops it on hearing this,
    /// before it is aborted.
    pub ending: Notify,
    tasks: Mutex<JoinSet<()>>,
}

impl Run {
    pub(crate) fn new(scope: Arc<ScopeHandle>, proxy: Handle, begin: Begin, ignore: Vec<Vec<String>>) -> Self {
        Run {
            scope,
            proxy,
            begin,
            ignore,
            pairs: Pairs::new(),
            over: Notify::new(),
            ending: Notify::new(),
            tasks: Mutex::new(JoinSet::new()),
        }
    }

    /// One more task of this run.
    pub(crate) async fn spawn(&self, task: impl Future<Output = ()> + Send + 'static) {
        self.tasks.lock().await.spawn(task);
    }

    /// End every task: tell them the run is ending, abort them, and
    /// wait for the aborts.
    ///
    /// Aborted rather than drained, because a task waiting on the
    /// caller — a fate that never comes, a channel nobody finishes —
    /// would otherwise hold the finish hostage. The notice first is
    /// for the tasks that have something to say to the proxy on the
    /// way out.
    pub(crate) async fn shutdown(&self) {
        self.ending.notify_waiters();
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
