//! The image cache: what podman holds, counted, and trimmed by the
//! provider since podman trims nothing.

use std::collections::{HashMap, HashSet};

use tokio::sync::Mutex;

use super::Error;
use crate::host::tools::podman;

/// The bookkeeping `image_cache_disk` is enforced with.
///
/// # What podman does not do
///
/// Its image store has no size cap and records no last use, and its
/// prune removes what is unused wholesale. So the provider keeps what
/// podman does not: which images are running now, which were used
/// most recently, and whether a pull of its own is in flight. An
/// image's size is not known before it is pulled — a reference is a
/// name until podman has it, and layers are shared, so a pull adds
/// less than the image's own size — which is why the store is
/// measured AFTER each pull and not before.
///
/// # The count
///
/// `podman image ls` lists every image with every layer counted, so a
/// layer two images share is counted twice; the count is the store's
/// bytes or more, never less, and the cap is met early rather than
/// passed.
///
/// # What is kept
///
/// An image whose config carries the label
/// [`KEEP_LABEL`](podman::KEEP_LABEL), `diverge.network/keep`, is
/// never removed and always counted, however it came to be in the
/// store. Nothing else is spared: not an image here before the
/// provider started, not one another program put here.
///
/// # What is removed
///
/// Whenever the count passes the cap, one image after another until
/// it does not: an image the provider never recorded — here before
/// it started, or put here by another program — before any it did,
/// oldest first; then the least recently used; and never one a
/// container runs, never one kept, never the one about to run. An
/// image the provider has not recorded may also be the one a pull of
/// its own is bringing in, and the pull's end is what records it, so
/// while any pull is in flight no unrecorded image is removed, which
/// is what keeps a trim from removing another deploy's image between
/// its arrival and its record. A store still over the cap after that
/// removes the image about to run, unless it is kept or running, and
/// the run is refused. An image podman will not remove — one a
/// container of some other program runs — is passed over, and stays
/// counted.
#[derive(Debug)]
pub struct Images {
    /// The most the count may reach, in BYTES.
    cap: u64,
    /// What the provider recorded.
    state: Mutex<State>,
    /// The measure-and-trim, taken by one deploy at a time, so two
    /// deploys reading one count never both remove for it; and taken
    /// by a pull's end, so its record is not missed by a trim already
    /// under way.
    trim: Mutex<()>,
}

/// What the provider recorded of the store, under one lock.
#[derive(Debug, Default)]
struct State {
    /// A clock that only goes up: one per use.
    tick: u64,
    /// How many pulls of the provider's are in flight.
    pulls: usize,
    /// How many containers run each image the provider recorded.
    running: HashMap<String, usize>,
    /// When each recorded image was last used — arrived, or started
    /// a run — as a tick of [`tick`](Self::tick).
    last_used: HashMap<String, u64>,
}

impl Images {
    /// The cache of `cap` bytes, with nothing recorded.
    pub fn new(cap: u64) -> Self {
        Images {
            cap,
            state: Mutex::new(State::default()),
            trim: Mutex::new(()),
        }
    }

    /// A pull of the provider's starts: until it ends, no unrecorded
    /// image is removed.
    pub async fn pull_begins(&self) {
        self.state.lock().await.pulls += 1;
    }

    /// A pull of the provider's ends, with `id` the image it brought
    /// in, or `None` where it brought none. Waits out a trim under
    /// way, so the image is recorded before any trim that could see
    /// it unrecorded with no pull in flight.
    pub async fn pull_ends(&self, id: Option<&str>) {
        let _trim = self.trim.lock().await;
        let mut state = self.state.lock().await;
        if let Some(id) = id {
            state.touch(id);
        }
        state.pulls = state.pulls.saturating_sub(1);
    }

    /// An image with `id` is about to run a container: measure the
    /// store, trim it, and record the run. `Err` is a store still over
    /// the cap, with the image removed where it could be, and no run
    /// recorded.
    pub async fn starting(&self, id: &str) -> Result<(), Error> {
        let _trim = self.trim.lock().await;
        let strangers = {
            let mut state = self.state.lock().await;
            state.touch(id);
            state.pulls == 0
        };
        let mut listed = podman::images().await.map_err(Error::Podman)?;
        let mut kept = HashSet::from([id.to_string()]);
        while listed.iter().map(|image| image.size).sum::<u64>() > self.cap {
            let Some(victim) = self.state.lock().await.victim(&listed, &kept, strangers) else {
                break;
            };
            let removed = podman::podman(["rmi", &victim])
                .await
                .map_err(Error::Podman)?
                .require("podman", |status| status.success())
                .is_ok();
            if removed {
                self.state.lock().await.forget(&victim);
                listed.retain(|image| image.id != victim);
            } else {
                kept.insert(victim);
            }
        }
        let mut state = self.state.lock().await;
        if listed.iter().map(|image| image.size).sum::<u64>() > self.cap {
            if listed.iter().any(|image| image.id == id && state.removable(image, strangers)) {
                let _ = podman::podman(["rmi", id]).await;
                state.forget(id);
            }
            return Err(Error::ImageCache);
        }
        *state.running.entry(id.to_string()).or_insert(0) += 1;
        Ok(())
    }

    /// An image with `id` is running one container fewer.
    pub async fn ended(&self, id: &str) {
        if let Some(count) = self.state.lock().await.running.get_mut(id) {
            *count = count.saturating_sub(1);
        }
    }
}

impl State {
    /// The image to remove next, among `listed` and not among `kept`:
    /// of those removable, an unrecorded one before any recorded,
    /// oldest first, then the least recently used, if any.
    fn victim(&self, listed: &[podman::Listed], kept: &HashSet<String>, strangers: bool) -> Option<String> {
        listed
            .iter()
            .filter(|image| !kept.contains(&image.id) && self.removable(image, strangers))
            .map(|image| match self.last_used.get(&image.id) {
                Some(tick) => (true, i64::try_from(*tick).unwrap_or(i64::MAX), image.id.clone()),
                None => (false, image.created, image.id.clone()),
            })
            .min()
            .map(|(_, _, id)| id)
    }

    /// Whether the image may be removed: not kept, running nothing,
    /// and recorded — or, with no pull in flight, which `strangers`
    /// says, unrecorded too.
    fn removable(&self, image: &podman::Listed, strangers: bool) -> bool {
        !image.keep
            && self.running.get(&image.id).is_none_or(|count| *count == 0)
            && (strangers || self.last_used.contains_key(&image.id))
    }

    /// The image is used now: the last of all for the next trim.
    fn touch(&mut self, id: &str) {
        let now = self.tick;
        self.tick += 1;
        self.last_used.insert(id.to_string(), now);
    }

    /// The image is gone from the store.
    fn forget(&mut self, id: &str) {
        self.last_used.remove(id);
        self.running.remove(id);
    }
}
