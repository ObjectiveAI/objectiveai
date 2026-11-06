//! One identity's stored volumes, by name.

use diverge_sdk::shared;
use std::sync::{Arc, Weak};

use dashmap::DashMap;
use futures_util::future;
use tokio::fs::DirEntry;
use tokio::sync::{Mutex, OnceCell};

use super::{Place, Reservation, Scratch, Volume, mode, name};
use crate::config::volumes::Store;

/// The volumes one identity created, held by name.
///
/// Read from the stores once, the first time the identity is named,
/// and changed after that only by what the manager does: a create
/// inserts, a delete removes. Each name is its own entry with its own
/// lock. The fixed volumes are not here: they are the configuration's,
/// and the manager answers for them on each call. Held while anything
/// holds it or one of its volumes, and let go of after — see
/// [`settle`] — to be read again when the identity is named again.
#[derive(Debug)]
pub struct Identity {
    /// The identity, as the store's directory is named.
    client_identity: shared::Identity,
    /// Where this identity is entered, for each of its volumes to
    /// look again when a handle of it is let go of.
    owner: Owner,
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
    pub(super) fn new(client_identity: &shared::Identity, identities: Weak<Identities>) -> Self {
        Identity {
            client_identity: client_identity.clone(),
            owner: Owner {
                identities,
                client_identity: client_identity.clone(),
            },
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
    pub(super) async fn load(&self, reservation: &Arc<Reservation>, scratch: &Arc<Scratch>) {
        self.loaded
            .get_or_init(|| async {
                future::join_all(
                    reservation
                        .stores()
                        .enumerate()
                        .map(|(index, store)| self.load_store(index, store, reservation, scratch)),
                )
                .await;
            })
            .await;
    }

    /// One store: its entries under this identity are read, and every
    /// one is examined at once. A directory that cannot be read holds
    /// no volumes.
    async fn load_store(&self, index: usize, store: &Store, reservation: &Arc<Reservation>, scratch: &Arc<Scratch>) {
        let Ok(mut entries) = tokio::fs::read_dir(store.path.join(directory(&self.client_identity))).await else {
            return;
        };
        let mut found = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            found.push(entry);
        }
        future::join_all(found.into_iter().map(|entry| self.load_entry(index, entry, reservation, scratch))).await;
    }

    /// One entry of a store: a stored volume when it is a regular
    /// file with a name a volume may have and its mode file beside
    /// it; nothing otherwise — an image without a mode file is not a
    /// volume, and a mode file is a dotfile, which no name passes.
    /// The file's type and the mode file are read at once.
    async fn load_entry(&self, index: usize, entry: DirEntry, reservation: &Arc<Reservation>, scratch: &Arc<Scratch>) {
        let Ok(name) = entry.file_name().into_string() else {
            return;
        };
        if !name::ok(&name) {
            return;
        }
        let path = entry.path();
        let mode_path = path.with_file_name(format!(".{name}"));
        let (kind, mode) = future::join(entry.file_type(), mode::read_mode(&mode_path)).await;
        if !kind.is_ok_and(|kind| kind.is_file()) {
            return;
        }
        let Ok(Some(mode)) = mode else {
            return;
        };
        self.volumes.insert(
            name.clone(),
            Volume::new(
                &name,
                Place::Stored {
                    store: index,
                    image: path,
                    reservation: Arc::clone(reservation),
                    scratch: Arc::clone(scratch),
                },
                mode.mode,
                Some(self.owner.clone()),
            ),
        );
    }

    /// Where this identity is entered: what a volume made for it is
    /// handed, so that letting go of the volume looks at the entry.
    pub(super) fn owner(&self) -> Owner {
        self.owner.clone()
    }

    /// Whether nothing holds any of this identity's volumes but the
    /// identity itself: no run, FUSE serve, operation, watch or
    /// container has one, and so none is held, attached or locked.
    fn idle(&self) -> bool {
        self.volumes.iter().all(|entry| entry.value().alone())
    }

    /// The identity.
    pub fn client_identity(&self) -> &shared::Identity {
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

/// The directory under a store a caller's volumes are kept in: its
/// issuer's and then its subject's
/// [`path_name`](diverge_sdk::identities::path_name), two levels, so
/// that every identity is a directory of its own whatever its issuer
/// and its subject hold.
/// Every identity that has asked and is still held, by identity.
pub type Identities = DashMap<shared::Identity, Arc<Identity>>;

/// Where an identity is entered, for a volume of it to look again.
#[derive(Debug, Clone)]
pub struct Owner {
    identities: Weak<Identities>,
    client_identity: shared::Identity,
}

impl Owner {
    /// Look at the entry again, once the handle being let go of is
    /// gone: [`settle`]d on a task of its own, since a drop runs while
    /// its own handle is still counted. With no runtime there is no
    /// provider left to keep anything for.
    pub(super) fn look_again(&self) {
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let owner = self.clone();
            runtime.spawn(async move {
                if let Some(identities) = owner.identities.upgrade() {
                    settle(&identities, &owner.client_identity);
                }
            });
        }
    }
}

/// Let go of the identity's entry when nothing holds it.
///
/// The rules, checked under the entry's own lock in the map, so no
/// handle is taken out between the look and the removal:
///
/// 1. Something else holds the [`Identity`] — a call under way, a load
///    running — and the entry stays.
/// 2. Something holds one of its volumes — a run, a FUSE serve, an
///    operation, a watch, a container bound to it — and the entry
///    stays: the volume's hold and its loop mount live in that handle,
///    and a second copy of the volume loaded beside it would hold
///    neither.
/// 3. Else the entry goes. The identity's volumes are on disk and are
///    read again the next time it is named.
///
/// What looks: every handle of an [`Identity`] the manager hands out,
/// as it is let go of, and every handle of one of its volumes, as it
/// is let go of — so an identity nothing holds any more is never kept.
pub fn settle(identities: &Identities, client_identity: &shared::Identity) {
    identities.remove_if(client_identity, |_, identity| Arc::strong_count(identity) == 1 && identity.idle());
}

pub fn directory(identity: &shared::Identity) -> std::path::PathBuf {
    std::path::PathBuf::from(diverge_sdk::identities::path_name(&identity.issuer)).join(diverge_sdk::identities::path_name(&identity.subject))
}
