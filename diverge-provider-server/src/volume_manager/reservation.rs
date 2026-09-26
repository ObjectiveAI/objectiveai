//! The stores, and the bytes reserved in each.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::future;
use tokio::fs;
use tokio::sync::OnceCell;

use super::name;
use crate::config::volumes::Store;

/// The stores volumes are created in, and how many bytes each has
/// given out, kept in memory and taken by compare-and-swap.
///
/// Shared by the manager, which creates, and by every stored volume,
/// which may grow or shrink: each asks a store for bytes with
/// [`reserve`](Self::reserve) and gives them back with
/// [`release`](Self::release), and neither waits on anything or reads
/// the disk to do it — a reservation is one atomic update of the
/// store's count, so two creates in two stores never touch the same
/// word, and two in one store race the word and never each other.
///
/// # One scan, then the count
///
/// A store's count is filled the first time the store is asked, by
/// one scan of every image in it — every identity's directory beside
/// every other, every image's length beside every other — and never
/// scanned again. Every reservation and every release on the store
/// awaits that scan first, so no image is made or removed while it
/// runs, and what it sees is what was on disk before the provider
/// touched the store. From then on the count is exact for everything
/// the provider does. It drifts only if an operator adds or removes
/// images under a store by hand while the provider runs: removed by
/// hand, the count stays high and the store under-commits, which is
/// the safe direction; added by hand, the count stays low until the
/// provider restarts. A store is the provider's, and hands stay off
/// it while the provider runs.
#[derive(Debug)]
pub struct Reservation {
    /// The stores, in the configuration's order of preference. Empty
    /// is a provider that creates none.
    stores: Vec<Slot>,
}

/// One store and its count.
#[derive(Debug)]
struct Slot {
    store: Store,
    /// The bytes every image in the store reserves between them, set
    /// by the one scan.
    used: OnceCell<AtomicU64>,
}

impl Reservation {
    pub fn new(stores: Vec<Store>) -> Self {
        Reservation {
            stores: stores
                .into_iter()
                .map(|store| Slot {
                    store,
                    used: OnceCell::new(),
                })
                .collect(),
        }
    }

    /// The stores, in the configuration's order.
    pub fn stores(&self) -> impl Iterator<Item = &Store> {
        self.stores.iter().map(|slot| &slot.store)
    }

    /// The store at `index`.
    pub fn store(&self, index: usize) -> &Store {
        &self.stores[index].store
    }

    /// The store's count, scanned into being the first time it is
    /// asked for. Every caller during the scan waits for it rather
    /// than starting another.
    async fn used(&self, index: usize) -> &AtomicU64 {
        let slot = &self.stores[index];
        slot.used
            .get_or_init(|| async { AtomicU64::new(scan(&slot.store.path).await) })
            .await
    }

    /// Take `bytes` from the store at `index`: `true` is the bytes
    /// taken, and the caller holds them until it
    /// [`release`](Self::release)s them; `false` is a store without
    /// the room, and nothing changed. One compare-and-swap.
    pub async fn reserve(&self, index: usize, bytes: u64) -> bool {
        let capacity = self.stores[index].store.capacity;
        self.used(index)
            .await
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                (capacity.saturating_sub(used) >= bytes).then(|| used + bytes)
            })
            .is_ok()
    }

    /// Give `bytes` back to the store at `index`. Never takes the
    /// count below zero.
    pub async fn release(&self, index: usize, bytes: u64) {
        let _ = self
            .used(index)
            .await
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| Some(used.saturating_sub(bytes)));
    }

    /// How many bytes the store at `index` has left: its capacity less
    /// its count, as of now. Never below `0`.
    pub async fn room(&self, index: usize) -> u64 {
        let capacity = self.stores[index].store.capacity;
        capacity.saturating_sub(self.used(index).await.load(Ordering::Acquire))
    }

    /// Every store's room, each asked beside every other, in the
    /// configuration's order: the first call's scans run at once.
    pub async fn rooms(&self) -> Vec<u64> {
        future::join_all((0..self.stores.len()).map(|index| self.room(index))).await
    }

    /// Take `bytes` from the first store, in the configuration's
    /// order of preference, that has them: its index, with the bytes
    /// TAKEN, or `None` with nothing taken anywhere.
    pub async fn reserve_first(&self, bytes: u64) -> Option<usize> {
        for index in 0..self.stores.len() {
            if self.reserve(index, bytes).await {
                return Some(index);
            }
        }
        None
    }
}

/// The bytes every image in the store reserves between them: every
/// identity's directory read beside every other, every image's length
/// beside every other. A store that does not exist reserves nothing.
async fn scan(store: &Path) -> u64 {
    let Ok(mut entries) = fs::read_dir(store).await else {
        return 0;
    };
    let mut identities = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        identities.push(entry.path());
    }
    future::join_all(identities.iter().map(|identity| scan_identity(identity)))
        .await
        .into_iter()
        .sum()
}

/// The bytes every image under one identity's directory reserves. A
/// directory that cannot be read reserves nothing.
async fn scan_identity(dir: &Path) -> u64 {
    let Ok(mut entries) = fs::read_dir(dir).await else {
        return 0;
    };
    // Only what could be an image: a mode file is a dotfile, and no
    // volume's name is one, so it is never billed.
    let mut images = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        if entry.file_name().to_str().is_some_and(name::ok) {
            images.push(entry.path());
        }
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
