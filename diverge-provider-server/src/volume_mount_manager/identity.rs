//! One identity's stored volumes, by name.

use std::sync::Arc;

use dashmap::DashMap;
use futures_util::future;
use tokio::fs::DirEntry;
use tokio::sync::{Mutex, OnceCell};

use super::{Place, Reservation, Volume, name};
use crate::config::volumes::Store;

/// The volumes one identity created, held by name.
///
/// Read from the stores once, the first time the identity is named,
/// and changed after that only by what the manager does: a create
/// inserts, a delete removes. Each name is its own entry with its own
/// lock. The fixed volumes are not here: they are the configuration's,
/// and the manager answers for them on each call.
#[derive(Debug)]
pub struct Identity {
    /// The identity, as the store's directory is named.
    client_identity: String,
    /// Set once the stores have been read for this identity, so a
    /// second caller waits for the first read rather than starting one.
    loaded: OnceCell<()>,
    /// Held across a create or a delete, so two of one name cannot
    /// race: the name is checked and the volume made, or removed,
    /// under it.
    namespace: Mutex<()>,
    volumes: DashMap<String, Volume>,
}

impl Identity {
    pub(super) fn new(client_identity: &str) -> Self {
        Identity {
            client_identity: client_identity.to_string(),
            loaded: OnceCell::new(),
            namespace: Mutex::new(()),
            volumes: DashMap::new(),
        }
    }

    /// Read the identity's volumes from the reservation's stores,
    /// once.
    ///
    /// In every store, `<store>/<identity>/` is read, every store
    /// beside every other and every entry of a store beside every
    /// other: every regular file whose name a volume may have is a
    /// stored volume, and anything else is passed over. A store, or
    /// the identity's directory in it, that does not exist holds no
    /// volumes. Each volume is handed the reservation, so it can ask
    /// its store for room when it grows.
    pub(super) async fn load(&self, reservation: &Arc<Reservation>) {
        self.loaded
            .get_or_init(|| async {
                future::join_all(
                    reservation
                        .stores()
                        .enumerate()
                        .map(|(index, store)| self.load_store(index, store, reservation)),
                )
                .await;
            })
            .await;
    }

    /// One store: its entries under this identity are read, and every
    /// one is examined at once. A directory that cannot be read holds
    /// no volumes.
    async fn load_store(&self, index: usize, store: &Store, reservation: &Arc<Reservation>) {
        let Ok(mut entries) = tokio::fs::read_dir(store.path.join(&self.client_identity)).await else {
            return;
        };
        let mut found = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            found.push(entry);
        }
        future::join_all(found.into_iter().map(|entry| self.load_entry(index, entry, reservation))).await;
    }

    /// One entry of a store: a stored volume when it is a regular
    /// file with a name a volume may have; nothing otherwise.
    async fn load_entry(&self, index: usize, entry: DirEntry, reservation: &Arc<Reservation>) {
        let Ok(name) = entry.file_name().into_string() else {
            return;
        };
        if !name::ok(&name) {
            return;
        }
        if !entry.file_type().await.is_ok_and(|kind| kind.is_file()) {
            return;
        }
        self.volumes.insert(
            name.clone(),
            Volume::new(
                &name,
                Place::Stored {
                    store: index,
                    image: entry.path(),
                    reservation: Arc::clone(reservation),
                },
            ),
        );
    }

    /// The identity, as the store's directory is named.
    pub fn client_identity(&self) -> &str {
        &self.client_identity
    }

    /// The lock a create or a delete holds — see the field.
    pub(super) fn namespace(&self) -> &Mutex<()> {
        &self.namespace
    }

    /// The volume under `name`, if the identity has one.
    pub fn volume(&self, name: &str) -> Option<Volume> {
        self.volumes.get(name).map(|volume| volume.clone())
    }

    /// Every volume the identity has, in no particular order.
    pub fn volumes(&self) -> Vec<Volume> {
        self.volumes.iter().map(|entry| entry.value().clone()).collect()
    }

    /// Enter a volume under its name. `false` is a name already held,
    /// and the volume handed in is not entered.
    pub fn insert(&self, volume: Volume) -> bool {
        match self.volumes.entry(volume.name().to_string()) {
            dashmap::Entry::Occupied(_) => false,
            dashmap::Entry::Vacant(vacant) => {
                vacant.insert(volume);
                true
            }
        }
    }

    /// Take the volume under `name` out of the identity, if there is
    /// one. The image is untouched: removing what is on disk is the
    /// manager's, and comes after.
    pub fn remove(&self, name: &str) -> Option<Volume> {
        self.volumes.remove(name).map(|(_, volume)| volume)
    }
}
