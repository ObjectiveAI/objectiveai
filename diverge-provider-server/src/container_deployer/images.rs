//! The image cache: what podman holds, counted, and trimmed by the
//! provider since podman trims nothing.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

use dashmap::DashMap;
use tokio::sync::Mutex;

use super::Error;
use crate::tools::podman;

/// The bookkeeping `image_cache_disk` is enforced with.
///
/// # What podman does not do
///
/// Its image store has no size cap and records no last use, and its
/// prune removes what is unused wholesale. So the provider keeps what
/// podman does not: which images were here when it started, which are
/// running now, and which ran most recently. An image's size is not
/// known before it is pulled — a reference is a name until podman has
/// it, and layers are shared, so a pull adds less than the image's
/// own size — which is why the store is measured AFTER each pull and
/// not before.
///
/// # The count
///
/// `podman image ls` lists every image with every layer counted, so a
/// layer two images share is counted twice; the count is the store's
/// bytes or more, never less, and the cap is met early rather than
/// passed.
///
/// # What is removed
///
/// The least recently run image that no container runs and that was
/// not here when the provider started, and the next, until the count
/// is under the cap or none remains. An image the provider has never
/// recorded — one another deploy pulled and has not yet recorded —
/// is not removable either, which is what keeps two deploys from
/// removing each other's image between the pull and the record. A
/// store still over the cap after that removes the image just
/// pulled, unless it is protected or running, and the run is refused.
/// An image podman will not remove — one a container of some other
/// program runs — is passed over, and stays counted.
#[derive(Debug)]
pub struct Images {
    /// The most the count may reach, in BYTES.
    cap: u64,
    /// Every image present when the provider started: never removed,
    /// always counted.
    protected: HashSet<String>,
    /// How many containers run each image the provider recorded.
    running: DashMap<String, usize>,
    /// When each recorded image last started a run, as a tick of
    /// [`tick`](Self::tick).
    last_used: DashMap<String, u64>,
    /// A clock that only goes up: one per record.
    tick: AtomicU64,
    /// The measure-and-trim, taken by one deploy at a time, so two
    /// deploys reading one count never both remove for it.
    trim: Mutex<()>,
}

impl Images {
    /// The cache of `cap` bytes, with `protected` the ids present now.
    pub fn new(cap: u64, protected: Vec<String>) -> Self {
        Images {
            cap,
            protected: protected.into_iter().collect(),
            running: DashMap::new(),
            last_used: DashMap::new(),
            tick: AtomicU64::new(0),
            trim: Mutex::new(()),
        }
    }

    /// An image with `id` is about to run a container: measure the
    /// store, trim it, and record the run. `Err` is a store still over
    /// the cap, with the image removed where it could be, and no run
    /// recorded.
    pub async fn starting(&self, id: &str) -> Result<(), Error> {
        let _trim = self.trim.lock().await;
        let mut listed = podman::images().await.map_err(Error::Podman)?;
        let mut kept = HashSet::new();
        while listed.iter().map(|image| image.size).sum::<u64>() > self.cap {
            let Some(victim) = self.victim(&listed, &kept) else {
                break;
            };
            let removed = podman::podman(["rmi", &victim])
                .await
                .map_err(Error::Podman)?
                .require("podman", |status| status.success())
                .is_ok();
            if removed {
                self.forget(&victim);
                listed.retain(|image| image.id != victim);
            } else {
                kept.insert(victim);
            }
        }
        if listed.iter().map(|image| image.size).sum::<u64>() > self.cap {
            if self.removable(id) {
                let _ = podman::podman(["rmi", id]).await;
                self.forget(id);
            }
            return Err(Error::ImageCache);
        }
        self.record(id);
        Ok(())
    }

    /// An image with `id` is running one container fewer.
    pub fn ended(&self, id: &str) {
        if let Some(mut count) = self.running.get_mut(id) {
            *count = count.saturating_sub(1);
        }
    }

    /// The image to remove next, among `listed` and not among `kept`:
    /// the least recently run of those removable, if any.
    fn victim(&self, listed: &[podman::Listed], kept: &HashSet<String>) -> Option<String> {
        listed
            .iter()
            .filter(|image| self.removable(&image.id) && !kept.contains(&image.id))
            .filter_map(|image| self.last_used.get(&image.id).map(|tick| (*tick, image.id.clone())))
            .min()
            .map(|(_, id)| id)
    }

    /// Whether the image may be removed: recorded, not protected, and
    /// running nothing.
    fn removable(&self, id: &str) -> bool {
        !self.protected.contains(id)
            && self.last_used.contains_key(id)
            && self.running.get(id).is_none_or(|count| *count == 0)
    }

    /// The image starts a run now.
    fn record(&self, id: &str) {
        let tick = self.tick.fetch_add(1, Ordering::AcqRel);
        self.last_used.insert(id.to_string(), tick);
        *self.running.entry(id.to_string()).or_insert(0) += 1;
    }

    /// The image is gone from the store.
    fn forget(&self, id: &str) {
        self.last_used.remove(id);
        self.running.remove(id);
    }
}
