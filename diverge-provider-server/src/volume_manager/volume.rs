//! One volume: the SDK's `Volume`, and what is known about it.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use diverge_provider_sdk::endpoints::volumes::edit::server::response::Edit;
use diverge_provider_sdk::endpoints::volumes::list::server::response;
use diverge_provider_sdk::endpoints::volumes::stat::server::response::Stat;
use diverge_provider_sdk::server::volume;
use futures_util::future;
use tokio::fs;
use tokio::sync::Mutex;

use super::{Error, Reservation, Walked, image, resize, walk};

/// Where a volume is, which is also what kind it is.
#[derive(Debug, Clone)]
pub enum Place {
    /// A created volume: the image file, in the store at this index
    /// of the configuration's list, and the reservation to ask that
    /// store for room through when the volume grows.
    Stored {
        store: usize,
        image: PathBuf,
        reservation: Arc<Reservation>,
    },
    /// A fixed volume of the configuration: the directory, the size
    /// the configuration declares for it, and when the provider
    /// started, which is its creation time where the directory's
    /// filesystem records none.
    Fixed { root: PathBuf, bytes: u64, started: u64 },
}

/// One volume, shared: the SDK's `Volume`.
///
/// Cheap to clone and one underneath, so the identity's map and
/// every handle the manager's `get` hands out are the same volume
/// with the same lock. What is known about it is where it is, the
/// SDK's lock, and — once asked — what a walk found. Its size and
/// its creation time are not kept: they are read from the filesystem
/// each time a listing wants them, since the filesystem is where they
/// are recorded and nothing else has to be kept right.
///
/// # The lock
///
/// An atomic flag: [`lock`](volume::Volume::lock) sets it if it was
/// clear and answers whether it did, [`unlock`](volume::Volume::unlock)
/// clears it and answers whether it was set,
/// [`locked`](volume::Volume::locked) reads it.
/// None waits on anything. The SDK takes it before a mount, a stat, an
/// edit or a delete and gives it back after — see the trait for the
/// rule.
#[derive(Debug, Clone)]
pub struct Volume {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    name: String,
    place: Place,
    /// The SDK's lock: `true` while a run, a stat, an edit or a
    /// delete has the volume.
    lock: AtomicBool,
    /// What the last walk found, if one has happened since the
    /// volume was read or last mounted.
    walked: Mutex<Option<Walked>>,
}

impl Volume {
    pub fn new(name: &str, place: Place) -> Self {
        Volume {
            inner: Arc::new(Inner {
                name: name.to_string(),
                place,
                lock: AtomicBool::new(false),
                walked: Mutex::new(None),
            }),
        }
    }

    /// The name a listing gives it.
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// Where it is.
    pub fn place(&self) -> &Place {
        &self.inner.place
    }

    /// The volume as a listing reports it, read from the filesystem
    /// now.
    ///
    /// A stored volume's size is its image's length and its creation
    /// time the image's birth time, or the image's modification time
    /// where the filesystem records no birth. A fixed volume's size
    /// is what the configuration declares and its creation time the
    /// directory's birth time, or when this provider started where
    /// the filesystem records none.
    pub async fn listing(&self) -> io::Result<response::Volume> {
        match &self.inner.place {
            Place::Stored { image, .. } => {
                let meta = fs::metadata(image).await?;
                Ok(response::Volume {
                    name: self.inner.name.clone(),
                    bytes: meta.len(),
                    created: meta.created().or_else(|_| meta.modified()).map(seconds).unwrap_or(0),
                })
            }
            Place::Fixed { root, bytes, started } => {
                let meta = fs::metadata(root).await?;
                Ok(response::Volume {
                    name: self.inner.name.clone(),
                    bytes: *bytes,
                    created: meta.created().map(seconds).unwrap_or(*started),
                })
            }
        }
    }

    /// What a walk finds: the bytes in use and the hash of the content.
    ///
    /// Walked once and held: a second check answers from the first,
    /// and a check that arrives while a walk runs waits for that walk
    /// rather than starting another. Held until the volume is
    /// [`mounted`](Self::mounted). A walk that fails holds nothing,
    /// and the next check walks again. A stored volume is walked
    /// inside its image, a fixed one in its directory.
    pub async fn check(&self) -> Result<Walked, Error> {
        let mut walked = self.inner.walked.lock().await;
        if let Some(walked) = walked.as_ref() {
            return Ok(walked.clone());
        }
        let found = match &self.inner.place {
            Place::Stored { image, .. } => image::walk_image(image).await?,
            Place::Fixed { root, .. } => walk::walk_directory(root).await?,
        };
        *walked = Some(found.clone());
        Ok(found)
    }

    /// The volume is being mounted into a container: whatever a walk
    /// found is forgotten, since the container may write from now on
    /// and nothing reports when. The next [`check`](Self::check) walks
    /// again. Separate from the SDK's lock, which a stat takes too:
    /// a stat must not wipe the walk it just made.
    pub async fn mounted(&self) {
        *self.inner.walked.lock().await = None;
    }

    /// Shrink the image from `current` to `bytes`: the content must
    /// fit, the filesystem is shrunk first, and the file after it, so
    /// the file is never shorter than the filesystem in it; the
    /// difference goes back to the store. A refusal by the tools
    /// leaves both as they were.
    async fn shrink(&self, store: usize, reservation: &Reservation, image: &Path, current: u64, bytes: u64) -> Result<Edit, Error> {
        if self.check().await?.bytes_used > bytes {
            return Ok(Edit::ContentTooLarge);
        }
        resize::resize(image, bytes).await?;
        set_len(image, bytes).await?;
        reservation.release(store, current - bytes).await;
        Ok(Edit::Edited)
    }

    /// Grow the image from `current` to `bytes`: the difference is
    /// taken from the store, by one atomic update and no I/O, the
    /// file is lengthened, and the filesystem after it. A failure
    /// after the bytes were taken puts the file back at `current`
    /// and gives them back.
    async fn grow(&self, store: usize, reservation: &Reservation, image: &Path, current: u64, bytes: u64) -> Result<Edit, Error> {
        if !reservation.reserve(store, bytes - current).await {
            return Ok(Edit::InsufficientCapacity);
        }
        let grown = async {
            set_len(image, bytes).await?;
            resize::resize(image, bytes).await
        }
        .await;
        if let Err(error) = grown {
            let _ = set_len(image, current).await;
            reservation.release(store, bytes - current).await;
            return Err(error);
        }
        Ok(Edit::Edited)
    }
}

/// The image's length set to `bytes`: past its end, a hole, since the
/// file is sparse; short of it, the tail dropped.
async fn set_len(image: &Path, bytes: u64) -> Result<(), Error> {
    let file = fs::OpenOptions::new().write(true).open(image).await?;
    file.set_len(bytes).await?;
    Ok(())
}

/// A moment as seconds since the Unix epoch; a moment before it is
/// `0`.
fn seconds(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_secs()).unwrap_or(0)
}

impl volume::Volume for Volume {
    type Error = Error;

    /// The flag set, if it was clear. One compare-and-swap, so two
    /// takers at once cannot both succeed.
    fn lock(&self) -> bool {
        self.inner
            .lock
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// The flag cleared; `true` is a flag that was set.
    fn unlock(&self) -> bool {
        self.inner.lock.swap(false, Ordering::AcqRel)
    }

    fn locked(&self) -> bool {
        self.inner.lock.load(Ordering::Acquire)
    }

    /// The listing and the walk, at once.
    async fn stat(&self) -> Result<Stat, Error> {
        let (volume, walked) = future::try_join(
            async { self.listing().await.map_err(Error::Io) },
            self.check(),
        )
        .await?;
        Ok(Stat {
            volume,
            bytes_used: walked.bytes_used,
            dirhash: walked.dirhash,
        })
    }

    /// The image resized, with the filesystem in it.
    ///
    /// Under the SDK's lock, so no container has the image. A fixed
    /// volume is refused, and so is a size the volume could not have
    /// been created at — under 4 MiB, or over what the formatter
    /// addresses. The size it has already is answered `Edited` with
    /// nothing run. Smaller is a shrink: refused as
    /// `ContentTooLarge` when the walk's `bytes_used` exceeds the
    /// size, else the filesystem shrunk and then the file. Larger is
    /// a grow: refused as `InsufficientCapacity` when
    /// the store lacks the difference, else the file lengthened and
    /// then the filesystem. Either way the tools are the system's
    /// `e2fsck` and `resize2fs` — see [`resize`] for where — and a
    /// content that fits the size but whose metadata does not is
    /// their refusal, returned as the error, with the volume as it
    /// was. A resize changes no file's content, so what a walk found
    /// stands.
    async fn edit(&self, bytes: u64) -> Result<Edit, Error> {
        let Place::Stored { store, image, reservation } = &self.inner.place else {
            return Err(Error::Fixed(self.inner.name.clone()));
        };
        // 4 MiB: less than that cannot hold ext4's journal beside
        // its metadata.
        if bytes < 4 * 1024 * 1024 {
            return Err(Error::TooSmall(bytes));
        }
        if u32::try_from(bytes / 4096).is_err() {
            return Err(Error::TooLarge(bytes));
        }
        let current = fs::metadata(image).await?.len();
        if bytes == current {
            return Ok(Edit::Edited);
        }
        if bytes < current {
            self.shrink(*store, reservation, image, current, bytes).await
        } else {
            self.grow(*store, reservation, image, current, bytes).await
        }
    }
}
