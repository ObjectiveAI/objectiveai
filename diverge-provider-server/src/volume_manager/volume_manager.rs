//! The manager: the stores, the fixed volumes, and every identity that
//! has asked.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use diverge_provider_sdk::endpoints::volumes::create::server::response::Creation;
use diverge_provider_sdk::endpoints::volumes::list::server::response;
use diverge_provider_sdk::server::volume_manager;
use futures_util::future;
use tokio::fs;
use tokio::sync::Mutex;

use super::{Error, Identity, Place, Volume, image, name};
use crate::config::volumes::{Fixed, HookInput, HookOutput, Store, Volumes};
use crate::hook;

/// The provider's volumes: the directories it offers every identity,
/// and everything done to them, over the stores it may create in and
/// the fixed volumes it holds already.
///
/// Starts knowing no identity and reads nothing: an identity enters
/// the map the first time it is named, and its stored volumes are
/// read from the stores then, once, and held for the manager's life.
/// Each identity is its own entry with its own lock, so two
/// identities never wait on each other. The fixed volumes are the
/// configuration's and are answered on each call, through their hook
/// where they have one.
#[derive(Debug)]
pub struct VolumeManager {
    /// Where volumes may be created, in the configuration's order of
    /// preference. Empty is a provider that creates none.
    stores: Vec<Store>,
    /// The volumes that exist already. Empty is a provider that
    /// holds none.
    fixed: Vec<Fixed>,
    /// The `hooks/` directory of the provider, where a fixed volume's
    /// `authorize_hook` is found by name.
    hooks_dir: PathBuf,
    /// When this provider started, in seconds since the Unix epoch:
    /// a fixed volume's creation time where its filesystem records
    /// none.
    started: u64,
    /// Every identity that has asked, by identity.
    identities: DashMap<String, Arc<Identity>>,
    /// Held across a create's capacity scan and its reservation, so
    /// two creates cannot both fit in the room one of them takes.
    reserving: Mutex<()>,
}

impl VolumeManager {
    /// A manager over the `volumes` section, absent or present, with
    /// the provider's `hooks/` directory.
    pub fn new(volumes: Option<Volumes>, hooks_dir: PathBuf) -> Self {
        let volumes = volumes.unwrap_or_default();
        VolumeManager {
            stores: volumes.stores.unwrap_or_default(),
            fixed: volumes.fixed.unwrap_or_default(),
            hooks_dir,
            started: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|elapsed| elapsed.as_secs())
                .unwrap_or(0),
            identities: DashMap::new(),
            reserving: Mutex::new(()),
        }
    }

    /// The identity's stored volumes, read from the stores the first
    /// time the identity is named and held from then on. Every caller
    /// that names the identity while its first load runs waits for
    /// that load rather than starting another.
    pub async fn identity(&self, client_identity: &str) -> Arc<Identity> {
        let identity = Arc::clone(
            &self
                .identities
                .entry(client_identity.to_string())
                .or_insert_with(|| Arc::new(Identity::new(client_identity))),
        );
        identity.load(&self.stores).await;
        identity
    }

    /// The fixed volume under `name`, if the configuration has one.
    fn fixed_named(&self, name: &str) -> Option<&Fixed> {
        self.fixed.iter().find(|fixed| fixed.name == name)
    }

    /// Whether `client_identity` may see the fixed volume: what its
    /// hook answers, run now, or `true` where it has none. A hook
    /// that does not answer — a manifest missing, a non-zero exit,
    /// an output that is not the expected one — is `false`, as the
    /// hook's output documents.
    async fn authorized(&self, client_identity: &str, fixed: &Fixed) -> bool {
        let Some(hook) = &fixed.authorize_hook else {
            return true;
        };
        let input = HookInput {
            identity: client_identity.to_string(),
            volume: fixed.name.clone(),
        };
        hook::run::<HookInput, HookOutput>(&self.hooks_dir, hook, &input)
            .await
            .is_ok_and(|output| output.authorized)
    }

    /// The fixed volume as a [`Volume`].
    fn fixed_volume(&self, fixed: &Fixed) -> Volume {
        Volume::new(
            &fixed.name,
            Place::Fixed {
                root: fixed.path.clone(),
                bytes: fixed.bytes,
                started: self.started,
            },
        )
    }

    /// Every fixed volume the identity may see, listed — each asked
    /// beside every other.
    async fn fixed_listing(&self, client_identity: &str) -> Result<Vec<response::Volume>, Error> {
        let listed = future::try_join_all(
            self.fixed
                .iter()
                .map(|fixed| self.fixed_one(client_identity, fixed)),
        )
        .await?;
        Ok(listed.into_iter().flatten().collect())
    }

    /// One fixed volume, listed if the identity may see it: the hook
    /// and the filesystem asked at once.
    async fn fixed_one(&self, client_identity: &str, fixed: &Fixed) -> Result<Option<response::Volume>, Error> {
        let (authorized, listing) = future::join(
            self.authorized(client_identity, fixed),
            self.fixed_volume(fixed).listing(),
        )
        .await;
        if !authorized {
            return Ok(None);
        }
        Ok(Some(listing?))
    }

    /// Every stored volume of the identity, listed — each read beside
    /// every other.
    async fn stored_listing(&self, identity: &Identity) -> Result<Vec<response::Volume>, Error> {
        let volumes = identity.volumes();
        let listed = future::try_join_all(volumes.iter().map(|volume| volume.listing())).await?;
        Ok(listed)
    }

    /// How many bytes the store has left: its capacity less the
    /// length of every image in it, of every identity, read from the
    /// filesystem now. Never below `0`.
    async fn room(&self, store: &Store) -> u64 {
        store.capacity.saturating_sub(used(&store.path).await)
    }

    /// Every store's room, each scanned beside every other, in the
    /// configuration's order.
    async fn rooms(&self) -> Vec<u64> {
        future::join_all(self.stores.iter().map(|store| self.room(store))).await
    }

    /// The first store, in the configuration's order of preference,
    /// with room for `bytes`: its index and itself.
    async fn store_with_room(&self, bytes: u64) -> Option<(usize, &Store)> {
        self.rooms()
            .await
            .into_iter()
            .position(|room| room >= bytes)
            .map(|index| (index, &self.stores[index]))
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

impl volume_manager::VolumeManager for VolumeManager {
    type Error = Error;
    type Volume = Volume;

    /// The fixed volumes the identity may see, then its stored ones,
    /// the two read at once.
    async fn list(&self, client_identity: &str) -> Result<Vec<response::Volume>, Error> {
        let identity = self.identity(client_identity).await;
        let (mut fixed, stored) = future::try_join(
            self.fixed_listing(client_identity),
            self.stored_listing(&identity),
        )
        .await?;
        fixed.extend(stored);
        Ok(fixed)
    }

    /// A fixed volume by that name, if the identity may see it, else
    /// the identity's stored volume by it. A run and a stat of a fixed
    /// volume both pass through its hook here.
    async fn get(&self, client_identity: &str, name: &str) -> Result<Option<Volume>, Error> {
        if let Some(fixed) = self.fixed_named(name) {
            if !self.authorized(client_identity, fixed).await {
                return Ok(None);
            }
            return Ok(Some(self.fixed_volume(fixed)));
        }
        Ok(self.identity(client_identity).await.volume(name))
    }

    /// The most room any one store has, as of now; `0` with no store.
    async fn create_capacity(&self, _client_identity: &str) -> Result<u64, Error> {
        Ok(self.rooms().await.into_iter().max().unwrap_or(0))
    }

    /// The image reserved in the first store with room, then formatted.
    ///
    /// The name is checked first, against what a name may be and
    /// against the fixed volumes, and the size against the least an
    /// ext4 filesystem can be. Then, under the identity's namespace
    /// lock: the name must be vacant; under the reservation lock as
    /// well, the stores are scanned in order and the first with room
    /// takes the image, made and sized; the reservation lock is let
    /// go, the image is formatted, and the volume is entered.
    async fn create(&self, client_identity: &str, name: &str, bytes: u64) -> Result<Creation, Error> {
        if !name::ok(name) || self.fixed_named(name).is_some() {
            return Err(Error::Name(name.to_string()));
        }
        // 4 MiB: less than that cannot hold ext4's journal beside
        // its metadata.
        if bytes < 4 * 1024 * 1024 {
            return Err(Error::TooSmall(bytes));
        }
        let identity = self.identity(client_identity).await;
        let _namespace = identity.namespace().lock().await;
        if identity.volume(name).is_some() {
            return Err(Error::Exists(name.to_string()));
        }
        let (index, path) = {
            let _reserving = self.reserving.lock().await;
            let Some((index, store)) = self.store_with_room(bytes).await else {
                return Ok(Creation::InsufficientCapacity);
            };
            fs::create_dir_all(store.path.join(client_identity)).await?;
            let path = image::image_path(&store.path, client_identity, name);
            image::reserve_image(&path, bytes).await?;
            (index, path)
        };
        image::format_image(&path, bytes).await?;
        identity.insert(Volume::new(name, Place::Stored { store: index, image: path }));
        Ok(Creation::Created)
    }

    /// The room left in the volume's own store, as of now; `0` for a
    /// fixed volume, which is never resized.
    async fn edit_capacity(&self, client_identity: &str, name: &str) -> Result<u64, Error> {
        let Some(volume) = self.get(client_identity, name).await? else {
            return Err(Error::Unknown(name.to_string()));
        };
        match volume.place() {
            Place::Stored { store, .. } => Ok(self.room(&self.stores[*store]).await),
            Place::Fixed { .. } => Ok(0),
        }
    }

    /// The volume taken out of the identity, then its image removed.
    /// An image already gone is still a deletion. A fixed volume is
    /// refused.
    async fn delete(&self, client_identity: &str, name: &str) -> Result<(), Error> {
        if self.fixed_named(name).is_some() {
            return Err(Error::Fixed(name.to_string()));
        }
        let identity = self.identity(client_identity).await;
        let _namespace = identity.namespace().lock().await;
        let Some(volume) = identity.remove(name) else {
            return Err(Error::Unknown(name.to_string()));
        };
        if let Place::Stored { image, .. } = volume.place() {
            match fs::remove_file(image).await {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(Error::Io(error)),
            }
        }
        Ok(())
    }
}
