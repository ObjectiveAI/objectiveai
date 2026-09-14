//! One identity's volumes, by name.

use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::OnceCell;

use super::{Place, Sidecar, Volume, sidecar};
use crate::config::volumes::{Fixed, Store};

/// The volumes one identity may name.
///
/// Read from the stores once, the first time the identity is named,
/// and changed after that only by what the manager does: a create
/// inserts, a delete removes. Each name is its own entry with its own
/// lock.
#[derive(Debug)]
pub struct Identity {
    /// The identity, as the store's directory is named.
    client_identity: String,
    /// Set once the stores have been read for this identity, so a
    /// second caller waits for the first read rather than starting one.
    loaded: OnceCell<()>,
    volumes: DashMap<String, Arc<Volume>>,
}

impl Identity {
    pub(super) fn new(client_identity: &str) -> Self {
        Identity {
            client_identity: client_identity.to_string(),
            loaded: OnceCell::new(),
            volumes: DashMap::new(),
        }
    }

    /// Read the identity's volumes from `stores` and `fixed`, once.
    ///
    /// In every store, `<store>/<identity>/` is read: every entry that
    /// is a directory with a sidecar beside it is a stored volume, and
    /// an entry with no sidecar, or one that will not read, is not a
    /// volume and is passed over. A store, or the identity's directory
    /// in it, that does not exist holds no volumes. Every fixed volume
    /// is entered as it is named in the configuration; whether this
    /// identity may see it is the manager's to ask, not the cache's to
    /// know. A stored volume and a fixed one of the same name are one
    /// entry, the stored one.
    pub(super) async fn load(&self, stores: &[Store], fixed: &[Fixed]) {
        self.loaded
            .get_or_init(|| async {
                for volume in fixed {
                    let created = created_of(&volume.path).await;
                    self.volumes.entry(volume.name.clone()).or_insert_with(|| {
                        Arc::new(Volume::new(
                            &volume.name,
                            volume.path.clone(),
                            Place::Fixed,
                            None,
                            created,
                        ))
                    });
                }
                for (store, place) in stores.iter().enumerate() {
                    let identity = place.path.join(&self.client_identity);
                    let Ok(mut entries) = tokio::fs::read_dir(&identity).await else {
                        continue;
                    };
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let Ok(name) = entry.file_name().into_string() else {
                            continue;
                        };
                        if !sidecar::name_ok(&name) {
                            continue;
                        }
                        if !entry.file_type().await.is_ok_and(|kind| kind.is_dir()) {
                            continue;
                        }
                        let Ok(sidecar) = Sidecar::read(&sidecar::path(&place.path, &self.client_identity, &name)).await
                        else {
                            continue;
                        };
                        self.volumes.insert(
                            name.clone(),
                            Arc::new(Volume::new(
                                &name,
                                entry.path(),
                                Place::Stored { store },
                                Some(sidecar.bytes),
                                sidecar.created,
                            )),
                        );
                    }
                }
            })
            .await;
    }

    /// The identity, as the store's directory is named.
    pub fn client_identity(&self) -> &str {
        &self.client_identity
    }

    /// The volume under `name`, if the identity has one.
    pub fn volume(&self, name: &str) -> Option<Arc<Volume>> {
        self.volumes.get(name).map(|volume| Arc::clone(&volume))
    }

    /// Every volume the identity has, in no particular order.
    pub fn volumes(&self) -> Vec<Arc<Volume>> {
        self.volumes.iter().map(|entry| Arc::clone(entry.value())).collect()
    }

    /// Enter a volume under its name. `false` is a name already held,
    /// and the volume handed in is not entered.
    pub fn insert(&self, volume: Arc<Volume>) -> bool {
        match self.volumes.entry(volume.name().to_string()) {
            dashmap::Entry::Occupied(_) => false,
            dashmap::Entry::Vacant(vacant) => {
                vacant.insert(volume);
                true
            }
        }
    }

    /// Take the volume under `name` out of the identity, if there is
    /// one. The directory is untouched: removing what is on disk is the
    /// manager's, and comes after.
    pub fn remove(&self, name: &str) -> Option<Arc<Volume>> {
        self.volumes.remove(name).map(|(_, volume)| volume)
    }
}

/// When a fixed volume came into being: the directory's creation time
/// where the filesystem reports one, else its modification time, else
/// `0`. In seconds since the Unix epoch.
async fn created_of(path: &std::path::Path) -> u64 {
    let Ok(meta) = tokio::fs::metadata(path).await else {
        return 0;
    };
    meta.created()
        .or_else(|_| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}
