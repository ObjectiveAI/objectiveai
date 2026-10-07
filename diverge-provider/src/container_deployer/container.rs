//! A container the deployer started, and its stop.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use diverge_sdk::provider::server::container;
use futures_util::future;
use tokio::process::Child;
use tokio::sync::Mutex;

use super::Shared;
use super::mounts::{Bound, release};
use crate::tools::podman;

/// A running container: its name in podman, where its proxy answers,
/// and everything its stop gives back.
#[derive(Debug)]
pub struct Container {
    /// The container's name in podman, `diverge-<uuid>`.
    pub(super) name: String,
    /// `ws://127.0.0.1:<port>`, the published port.
    pub(super) address: String,
    /// The id of the image it runs, for the image cache's count.
    pub(super) image: String,
    /// The `disk` it took from the cap.
    pub(super) disk: u64,
    /// The `memory` it took from the cap.
    pub(super) memory: u64,
    /// Its volume mounts, each to detach.
    pub(super) bound: Vec<Bound>,
    /// The `podman exec` running the proxy, held so the proxy's life
    /// is the container's and its end is seen; dropped at the stop.
    pub(super) proxy: Mutex<Option<Child>>,
    /// The caps and the image cache to give back to.
    pub(super) shared: Arc<Shared>,
    /// Whether the stop has happened: it happens once.
    pub(super) stopped: AtomicBool,
}

impl container::Container for Container {
    fn address(&self) -> &str {
        &self.address
    }

    /// Once: the container removed at once — killed, not asked — the
    /// proxy's exec dropped, every loop mount unmounted, and the
    /// disk, the memory and the image's run given back. A refusal
    /// along the way is not reported, since the trait has nowhere to
    /// report one; the container is gone as far as this end can make
    /// it.
    async fn stop(&self) {
        if self.stopped.swap(true, Ordering::AcqRel) {
            return;
        }
        let _ = podman::podman(["rm", "--force", "--time", "0", &self.name]).await;
        drop(self.proxy.lock().await.take());
        future::join(release(&self.bound), async {
            self.shared.images.ended(&self.image);
            self.shared.disk.give(self.disk);
            self.shared.memory.give(self.memory);
        })
        .await;
    }
}
