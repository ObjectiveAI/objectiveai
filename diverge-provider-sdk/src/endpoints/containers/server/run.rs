//! What every task serving one container scope shares.

use std::future::Future;
use std::sync::Arc;

use tokio::sync::{Mutex, Notify};
use tokio::task::JoinSet;

use super::pairs::Pairs;
use crate::server::container_client::ContainerClient;
use crate::server::scope_handle::ScopeHandle;

/// One scope on one container, as its tasks see it: the scope to
/// answer on, the client to the proxy, the database pairs in flight,
/// the tasks themselves, and the one signal that ends everything.
pub(crate) struct Run {
    /// The scope: every channel is opened and answered on it.
    pub scope: Arc<ScopeHandle>,
    /// The proxy inside the container, at the address the deploy
    /// reported — or, for a connector, the one the directory holds.
    pub client: ContainerClient,
    /// Every mount's path, which every filetree opened on the
    /// container leaves out.
    pub ignore: Vec<Vec<String>>,
    /// Database connections whose caller's half has not opened yet.
    pub pairs: Pairs,
    /// The container is gone — its `/requests` connection ended — or,
    /// for a connector, the run it joined is over. Whoever learns it
    /// says so here; the serve loop hears it.
    pub over: Notify,
    tasks: Mutex<JoinSet<()>>,
}

impl Run {
    pub(crate) fn new(scope: Arc<ScopeHandle>, client: ContainerClient, ignore: Vec<Vec<String>>) -> Self {
        Run {
            scope,
            client,
            ignore,
            pairs: Pairs::new(),
            over: Notify::new(),
            tasks: Mutex::new(JoinSet::new()),
        }
    }

    /// One more task of this scope's, to be waited for at the end.
    pub(crate) async fn spawn(&self, task: impl Future<Output = ()> + Send + 'static) {
        self.tasks.lock().await.spawn(task);
    }

    /// End every task and wait for them to be gone.
    ///
    /// Aborted, not drained: by now the container is stopped or left,
    /// so what a task was doing cannot complete — and a task waiting
    /// on the CALLER, for an answer that may never come, must not
    /// hold the scope's finish hostage. What an aborted task leaves
    /// is a channel the caller finishes on its own, or the connection
    /// ends.
    pub(crate) async fn shutdown(&self) {
        let mut tasks = self.tasks.lock().await;
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
    }

    /// One response on a channel the caller opened, if there is one
    /// to send.
    pub(crate) async fn respond(&self, channel: u32, payload: Option<Vec<u8>>) {
        if let Some(payload) = payload {
            self.scope.send_channel_response(channel, &payload).await;
        }
    }

    /// The channel's finish.
    pub(crate) async fn finish(&self, channel: u32) {
        self.scope.send_channel_response_finish(channel).await;
    }
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Run").field("scope", &self.scope).field("pairs", &self.pairs).finish_non_exhaustive()
    }
}

/// One frame on the main stream, if there is one to send.
pub(crate) async fn send(scope: &ScopeHandle, payload: Option<Vec<u8>>) {
    if let Some(payload) = payload {
        scope.send_response(&payload).await;
    }
}
