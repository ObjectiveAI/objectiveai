//! The stores, and the room left in each.

use std::path::Path;

use futures_util::future;
use tokio::fs;
use tokio::sync::{Mutex, MutexGuard};

use crate::config::volumes::Store;

/// The stores volumes are created in, and the one lock under which
/// bytes are reserved in them.
///
/// Shared by the manager, which creates, and by every stored volume,
/// which may grow: both ask a store how much room it has and take
/// some of it, and both do so under [`lock`](Self::lock), so two of
/// them cannot both fit in the room one of them takes. What a store
/// has left is never kept: it is its configured capacity less the
/// length of every image in it, read from the filesystem when asked,
/// since the filesystem is where the reservations are and nothing
/// else has to be kept right.
#[derive(Debug)]
pub struct Reservation {
    /// Where volumes may be created, in the configuration's order of
    /// preference. Empty is a provider that creates none.
    stores: Vec<Store>,
    /// Held across a capacity scan and the reservation it decides,
    /// which is the `set_len` that commits the bytes.
    lock: Mutex<()>,
}

impl Reservation {
    pub fn new(stores: Vec<Store>) -> Self {
        Reservation {
            stores,
            lock: Mutex::new(()),
        }
    }

    /// The stores, in the configuration's order.
    pub fn stores(&self) -> &[Store] {
        &self.stores
    }

    /// Take the lock, for the length of a scan and the reservation it
    /// decides.
    pub async fn lock(&self) -> MutexGuard<'_, ()> {
        self.lock.lock().await
    }

    /// How many bytes the store at `index` has left: its capacity less
    /// the length of every image in it, of every identity, read from
    /// the filesystem now. Never below `0`.
    pub async fn room(&self, index: usize) -> u64 {
        let store = &self.stores[index];
        store.capacity.saturating_sub(used(&store.path).await)
    }

    /// Every store's room, each scanned beside every other, in the
    /// configuration's order.
    pub async fn rooms(&self) -> Vec<u64> {
        future::join_all((0..self.stores.len()).map(|index| self.room(index))).await
    }

    /// The first store, in the configuration's order of preference,
    /// with room for `bytes`.
    pub async fn first_with_room(&self, bytes: u64) -> Option<usize> {
        self.rooms().await.into_iter().position(|room| room >= bytes)
    }
}

/// The bytes every image in the store reserves between them: every
/// identity's directory read beside every other, every image's length
/// beside every other. A store that does not exist reserves nothing.
async fn used(store: &Path) -> u64 {
    let Ok(mut entries) = fs::read_dir(store).await else {
        return 0;
    };
    let mut identities = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        identities.push(entry.path());
    }
    future::join_all(identities.iter().map(|identity| used_identity(identity)))
        .await
        .into_iter()
        .sum()
}

/// The bytes every image under one identity's directory reserves. A
/// directory that cannot be read reserves nothing.
async fn used_identity(dir: &Path) -> u64 {
    let Ok(mut entries) = fs::read_dir(dir).await else {
        return 0;
    };
    let mut images = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        images.push(entry.path());
    }
    future::join_all(images.iter().map(|image| image_length(image)))
        .await
        .into_iter()
        .sum()
}

/// One image's length, or `0` for anything that is not a regular
/// file.
async fn image_length(image: &Path) -> u64 {
    fs::metadata(image)
        .await
        .ok()
        .filter(|meta| meta.is_file())
        .map(|meta| meta.len())
        .unwrap_or(0)
}
