//! The manager: the stores, the fixed volumes, and every identity that
//! has asked.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use diverge_sdk::provider::endpoints::volumes::create::server::response::Creation;
use diverge_sdk::provider::endpoints::volumes::list::server::response;
use diverge_sdk::provider::server::volume_manager;
use futures_util::future;
use tokio::fs;

use diverge_sdk::provider::endpoints::volumes::Mode;

use super::{Error, Identity, ModeFile, Place, Reservation, Scratch, Volume, image, mode, name};
use crate::config::volumes::{Fixed, HookInput, HookOutput, Volumes};
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
    /// The stores, and the room left in each: shared with every
    /// stored volume, which asks it to grow.
    reservation: Arc<Reservation>,
    /// Where an ephemeral serve keeps its scratch, and the cap it
    /// counts against: shared with every stored volume.
    scratch: Arc<Scratch>,
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
}

impl VolumeManager {
    /// A manager over the `volumes` section, absent or present, with
    /// the provider's `hooks/` directory.
    pub fn new(volumes: Option<Volumes>, hooks_dir: PathBuf, scratch: Scratch) -> Self {
        let volumes = volumes.unwrap_or_default();
        VolumeManager {
            reservation: Arc::new(Reservation::new(volumes.stores.unwrap_or_default())),
            scratch: Arc::new(scratch),
            fixed: volumes.fixed.unwrap_or_default(),
            hooks_dir,
            started: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|elapsed| elapsed.as_secs())
                .unwrap_or(0),
            identities: DashMap::new(),
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
        identity.load(&self.reservation, &self.scratch).await;
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

    /// The fixed volume as a [`Volume`], in the mode the configuration
    /// declares.
    fn fixed_volume(&self, fixed: &Fixed) -> Volume {
        Volume::new(
            &fixed.name,
            Place::Fixed {
                root: fixed.path.clone(),
                bytes: fixed.bytes,
                started: self.started,
            },
            fixed.mode,
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
        Ok(self.reservation.rooms().await.into_iter().max().unwrap_or(0))
    }

    /// The bytes reserved in the first store with room, the image
    /// made and formatted there, the volume entered.
    ///
    /// The name is checked first, against what a name may be and
    /// against the fixed volumes, and the size against the least an
    /// ext4 filesystem can be. Then, under the identity's namespace
    /// lock — the identity's alone, so identities never wait on each
    /// other — the name must be vacant; the bytes are taken from the
    /// first store that has them, by one atomic update and no I/O;
    /// the image is made, sized and formatted; and the volume is
    /// entered. A failure after the bytes were taken gives them back.
    async fn create(&self, client_identity: &str, name: &str, bytes: u64, mode: Mode) -> Result<Creation, Error> {
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
        let Some(index) = self.reservation.reserve_first(bytes).await else {
            return Ok(Creation::InsufficientCapacity);
        };
        let store = self.reservation.store(index);
        let path = image::image_path(&store.path, client_identity, name);
        let mode_path = mode::mode_path(&store.path, client_identity, name);
        let made = async {
            fs::create_dir_all(store.path.join(client_identity)).await?;
            mode::write_mode(&mode_path, ModeFile { mode }).await?;
            make_image(&path, bytes).await
        }
        .await;
        if let Err(error) = made {
            let _ = future::join(fs::remove_file(&path), fs::remove_file(&mode_path)).await;
            self.reservation.release(index, bytes).await;
            return Err(error);
        }
        identity.insert(Volume::new(
            name,
            Place::Stored {
                store: index,
                image: path,
                reservation: Arc::clone(&self.reservation),
                scratch: Arc::clone(&self.scratch),
            },
            mode,
        ));
        Ok(Creation::Created)
    }

    /// The room left in the volume's own store, as of now; `0` for a
    /// fixed volume, which is never resized.
    async fn edit_capacity(&self, client_identity: &str, name: &str) -> Result<u64, Error> {
        let Some(volume) = self.get(client_identity, name).await? else {
            return Err(Error::Unknown(name.to_string()));
        };
        match volume.place() {
            Place::Stored { store, reservation, .. } => Ok(reservation.room(*store).await),
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
        if let Place::Stored { store, image, reservation, .. } = volume.place() {
            let length = image_length(image).await;
            // The image first, then its mode file: a crash between
            // the two leaves a mode file alone, which is nothing, and
            // never an image alone that a store would bill.
            remove(image).await?;
            let name = image.file_name().map(|name| name.to_string_lossy()).unwrap_or_default();
            remove(&image.with_file_name(format!(".{name}"))).await?;
            reservation.release(*store, length).await;
        }
        Ok(())
    }
}

/// The image made at `path`, its directory being there already:
/// reserved sparse, then formatted.
async fn make_image(path: &Path, bytes: u64) -> Result<(), Error> {
    image::reserve_image(path, bytes).await?;
    image::format_image(path, bytes).await
}

/// One file removed, a file already gone being nothing.
async fn remove(path: &Path) -> Result<(), Error> {
    match fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Error::Io(error)),
    }
}

/// The image's length, which a delete gives back to its store; an
/// image already gone gives back nothing.
async fn image_length(image: &Path) -> u64 {
    fs::metadata(image).await.map(|meta| meta.len()).unwrap_or(0)
}
