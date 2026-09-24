//! One volume: the SDK's `Volume`, and what is known about it.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use diverge_provider_sdk::CHUNK_SIZE;
use diverge_provider_sdk::endpoints::volumes::edit::client::request::Change;
use diverge_provider_sdk::endpoints::volumes::edit::server::response::Edit;
use diverge_provider_sdk::endpoints::volumes::list::server::response;
use diverge_provider_sdk::endpoints::volumes::stat::server::response::Stat;
use diverge_provider_sdk::server::holders::Holders;
use diverge_provider_sdk::server::volume;
use diverge_provider_sdk::shared::error;
use diverge_provider_sdk::shared::filetree::response::Node;
use futures_util::stream::{BoxStream, MapErr};
use futures_util::{Stream, StreamExt as _, TryStreamExt as _, future};
use tokio::fs;
use tokio::io::AsyncReadExt as _;
use tokio::sync::{Mutex, mpsc};

use super::{Error, Mode, Reservation, Walked, image, mode, walk};
use crate::tools::{mount, resize};

/// The hold's count when the one exclusive holder has it: every
/// other value is how many containers have the volume mounted.
const LOCKED: u32 = u32::MAX;

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
/// with the same hold. What is known about it is where it is, the
/// SDK's hold, the one loop mount its image is on while any container
/// has it, and — once asked — what a walk found. Its size and its
/// creation time are not kept: they are read from the filesystem
/// each time a listing wants them, since the filesystem is where they
/// are recorded and nothing else has to be kept right.
///
/// # The hold
///
/// One atomic count: `0` is free, `n` is `n` containers mounting the
/// volume, and `LOCKED` is the one stat, edit or delete that has it
/// to itself. [`mount`](volume::Volume::mount) adds one unless it is
/// locked, [`unmount`](volume::Volume::unmount) takes one away,
/// [`lock`](volume::Volume::lock) goes from free to locked and
/// [`unlock`](volume::Volume::unlock) back, each one compare-and-swap
/// that never waits. The SDK takes the shared hold before a mount and
/// the exclusive one before a stat, an edit or a delete, and gives
/// each back after — see the trait for the rule.
///
/// # One loop mount, however many containers
///
/// A stored volume's image is an ext4 filesystem, and a filesystem
/// mounted twice read-write corrupts itself. So the image is
/// loop-mounted once, on the first [`attach`](Self::attach), and
/// every container that has the volume binds that one directory;
/// the last [`detach`](Self::detach) unmounts it. A fixed volume is
/// a directory already, and is bound as it is.
#[derive(Debug, Clone)]
pub struct Volume {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    name: String,
    place: Place,
    /// Whether the volume keeps what containers write into it: what
    /// its mode file holds, or the configuration declares for a fixed
    /// volume. Changed only under the exclusive hold, so no container
    /// is bound while it moves.
    persist: AtomicBool,
    /// The SDK's hold: free, some number of mounters, or [`LOCKED`].
    holders: AtomicU32,
    /// What the last walk found, if one has happened since the
    /// volume was read or last mounted.
    walked: Mutex<Option<Walked>>,
    /// The loop mount of a stored volume's image while any container
    /// has it: the directory, as the tool and podman see it, and how
    /// many containers bind it. Taken under this mutex so a first
    /// attach and a last detach never cross.
    attached: Mutex<Option<Attached>>,
}

/// A stored volume's image on its directory, and who binds it.
#[derive(Debug)]
struct Attached {
    /// The directory the image is loop-mounted on, as the tool and
    /// podman see it.
    dir: String,
    /// How many containers bind it.
    count: u32,
}

impl Volume {
    pub fn new(name: &str, place: Place, persist: bool) -> Self {
        Volume {
            inner: Arc::new(Inner {
                name: name.to_string(),
                place,
                persist: AtomicBool::new(persist),
                holders: AtomicU32::new(0),
                walked: Mutex::new(None),
                attached: Mutex::new(None),
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

    /// Whether the volume keeps what containers write into it, as of
    /// now: what a mount reads to bind the volume plainly or under an
    /// overlay.
    pub fn persist(&self) -> bool {
        self.inner.persist.load(Ordering::Acquire)
    }

    /// The volume as a listing reports it, read from the filesystem
    /// now.
    ///
    /// A stored volume's size is its image's length and its creation
    /// time the image's birth time, or the image's modification time
    /// where the filesystem records no birth. A fixed volume's size
    /// is what the configuration declares and its creation time the
    /// directory's birth time, or when this provider started where
    /// the filesystem records none. Either's persist mode is what
    /// this holds.
    pub async fn listing(&self) -> io::Result<response::Volume> {
        match &self.inner.place {
            Place::Stored { image, .. } => {
                let meta = fs::metadata(image).await?;
                Ok(response::Volume {
                    name: self.inner.name.clone(),
                    bytes: meta.len(),
                    created: meta.created().or_else(|_| meta.modified()).map(seconds).unwrap_or(0),
                    persist: self.persist(),
                })
            }
            Place::Fixed { root, bytes, started } => {
                let meta = fs::metadata(root).await?;
                Ok(response::Volume {
                    name: self.inner.name.clone(),
                    bytes: *bytes,
                    created: meta.created().map(seconds).unwrap_or(*started),
                    persist: self.persist(),
                })
            }
        }
    }

    /// What a walk finds: the bytes in use and the hash of the content.
    ///
    /// Walked once and held: a second check answers from the first,
    /// and a check that arrives while a walk runs waits for that walk
    /// rather than starting another. Held until the volume is
    /// [`attached`](Self::attach). A walk that fails holds nothing,
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

    /// The volume made ready to bind into one more container, and the
    /// directory to bind: for a stored volume, the one directory its
    /// image is loop-mounted on — mounted now, on `<mounts_dir>/<uuid>`
    /// as the tool sees it, when this is the first container to have
    /// it — and for a fixed volume its root, as the tool sees it.
    /// Whatever a walk found is forgotten, since a container may
    /// write from now on and nothing reports when; the next
    /// [`check`](Self::check) walks again. Every attach is matched by
    /// one [`detach`](Self::detach).
    pub async fn attach(&self, mounts_dir: &Path) -> Result<String, crate::tools::Error> {
        *self.inner.walked.lock().await = None;
        let image = match &self.inner.place {
            Place::Fixed { root, .. } => return Ok(tool_path(root)),
            Place::Stored { image, .. } => image,
        };
        let mut attached = self.inner.attached.lock().await;
        match attached.as_mut() {
            Some(attached) => {
                attached.count += 1;
                Ok(attached.dir.clone())
            }
            None => {
                let dir = tool_path(&mounts_dir.join(uuid::Uuid::new_v4().to_string()));
                mount::mount(image, &dir).await?;
                *attached = Some(Attached {
                    dir: dir.clone(),
                    count: 1,
                });
                Ok(dir)
            }
        }
    }

    /// One container fewer has the volume: the last to go unmounts a
    /// stored volume's image and removes the directory. A fixed
    /// volume has nothing to give back. A refusal by the tools is not
    /// reported, since there is nobody to report it to; the directory
    /// is swept at the next start.
    pub async fn detach(&self) {
        let mut attached = self.inner.attached.lock().await;
        let Some(current) = attached.as_mut() else {
            return;
        };
        current.count = current.count.saturating_sub(1);
        if current.count == 0 {
            let dir = current.dir.clone();
            *attached = None;
            let _ = mount::unmount(&dir).await;
        }
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
            resize::resize(image, bytes).await?;
            Ok::<(), Error>(())
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

/// A channel's receiver as a stream: each item as it arrives, ended
/// when the sender is gone.
fn receiver_stream<T: Send + 'static>(receiver: mpsc::Receiver<T>) -> impl Stream<Item = T> + Send + 'static {
    futures_util::stream::unfold(receiver, |mut receiver| async move {
        receiver.recv().await.map(|item| (item, receiver))
    })
}

/// Every piece of `content` forwarded down `sender`, until the
/// content ends or the receiver is gone.
async fn forward<S>(content: S, sender: mpsc::Sender<Result<Bytes, error::Error>>)
where
    S: Stream<Item = Result<Bytes, error::Error>> + Send + 'static,
{
    let mut content = std::pin::pin!(content);
    while let Some(piece) = content.next().await {
        if sender.send(piece).await.is_err() {
            break;
        }
    }
}

/// A fixed volume's file read out through `tokio::fs`, in pieces of
/// at most [`CHUNK_SIZE`], down `sender`: a zero-byte file one empty
/// piece, a failure last.
async fn read_host(mut file: fs::File, sender: mpsc::Sender<Result<Bytes, Error>>) {
    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut sent = 0u64;
    loop {
        let read = file.read(&mut buffer).await;
        let n = match read {
            Ok(n) => n,
            Err(error) => {
                let _ = sender.send(Err(Error::Io(error))).await;
                return;
            }
        };
        if n == 0 && sent > 0 {
            return;
        }
        sent += n as u64;
        if sender.send(Ok(Bytes::copy_from_slice(&buffer[..n]))).await.is_err() || n == 0 {
            return;
        }
    }
}

/// A fixed volume's file written in through `tokio::fs`: every
/// missing parent made, the content into a temporary beside the
/// destination, the temporary renamed over it; a failure removes the
/// temporary and leaves the destination.
async fn write_host<S>(root: &Path, path: &[String], content: S) -> Result<(), Error>
where
    S: Stream<Item = Result<Bytes, error::Error>> + Send + 'static,
{
    let (name, parents) = path.split_last().expect("the handler refused an empty path");
    let mut dir = root.to_path_buf();
    dir.extend(parents);
    fs::create_dir_all(&dir).await.map_err(|error| match error.kind() {
        std::io::ErrorKind::NotADirectory | std::io::ErrorKind::AlreadyExists => Error::NotADirectory(parents.to_vec()),
        _ => Error::Io(error),
    })?;
    let destination = dir.join(name);
    if fs::metadata(&destination).await.is_ok_and(|meta| !meta.is_file()) {
        return Err(Error::NotAFile(path.to_vec()));
    }
    let temporary = dir.join(format!(".{name}.{}", uuid::Uuid::new_v4()));
    let filled = fill_host(&temporary, content).await;
    let placed = match filled {
        Ok(()) => fs::rename(&temporary, &destination).await.map_err(Error::Io),
        Err(error) => Err(error),
    };
    if placed.is_err() {
        let _ = fs::remove_file(&temporary).await;
    }
    placed
}

/// The temporary at `temporary` made and filled with every piece of
/// `content`, in order; a piece that is the caller's error is
/// [`Error::Content`].
async fn fill_host<S>(temporary: &Path, content: S) -> Result<(), Error>
where
    S: Stream<Item = Result<Bytes, error::Error>> + Send + 'static,
{
    use tokio::io::AsyncWriteExt as _;
    let mut file = fs::File::create(temporary).await?;
    let mut content = std::pin::pin!(content);
    while let Some(piece) = content.next().await {
        let bytes = piece.map_err(Error::Content)?;
        file.write_all(&bytes).await?;
    }
    file.flush().await?;
    Ok(())
}

/// The mode file beside a stored volume's image: the dotfile of the
/// image's own name, in the image's directory — [`mode_path`] for a
/// path already built.
///
/// [`mode_path`]: super::mode_path
fn mode_file(image: &Path) -> PathBuf {
    let name = image.file_name().map(|name| name.to_string_lossy()).unwrap_or_default();
    image.with_file_name(format!(".{name}"))
}

/// The image's length set to `bytes`: past its end, a hole, since the
/// file is sparse; short of it, the tail dropped.
async fn set_len(image: &Path, bytes: u64) -> Result<(), Error> {
    let file = fs::OpenOptions::new().write(true).open(image).await?;
    file.set_len(bytes).await?;
    Ok(())
}

/// A host path as the tool and podman see it: on Linux, the path
/// itself.
#[cfg(target_os = "linux")]
fn tool_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// A host path as the tool and podman see it: the machine's view of
/// it.
#[cfg(not(target_os = "linux"))]
fn tool_path(path: &Path) -> String {
    crate::tools::podman::path(path)
}

/// A moment as seconds since the Unix epoch; a moment before it is
/// `0`.
fn seconds(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_secs()).unwrap_or(0)
}

impl volume::Volume for Volume {
    type Error = Error;

    /// One more mounter, unless the volume is locked. One
    /// compare-and-swap, so a lock and a mount at once cannot both
    /// succeed.
    async fn mount(&self) -> bool {
        self.inner
            .holders
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |holders| match holders {
                LOCKED => None,
                count => Some(count + 1),
            })
            .is_ok()
    }

    /// One mounter fewer; `false` is a volume with none, or locked.
    async fn unmount(&self) -> bool {
        self.inner
            .holders
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |holders| match holders {
                0 | LOCKED => None,
                count => Some(count - 1),
            })
            .is_ok()
    }

    /// Free to locked, and nothing else: a volume anyone has is
    /// refused.
    async fn lock(&self) -> bool {
        self.inner
            .holders
            .compare_exchange(0, LOCKED, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Locked to free; `false` is a volume that was not locked.
    async fn unlock(&self) -> bool {
        self.inner
            .holders
            .compare_exchange(LOCKED, 0, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    async fn holders(&self) -> Holders {
        match self.inner.holders.load(Ordering::Acquire) {
            0 => Holders::Free,
            LOCKED => Holders::Locked,
            count => Holders::Mounted(count),
        }
    }

    /// A stored volume is watched by the container's proxy; a fixed
    /// one by the provider, being content the operator put there, as
    /// large and as still as they like, that a proxy has no business
    /// walking on every filetree.
    fn tree(&self) -> bool {
        matches!(self.inner.place, Place::Stored { .. })
    }

    /// The watch a fixed volume gets from the provider: its
    /// directory, walked and watched on this host — see
    /// [`watch`](crate::watch).
    type Watch = MapErr<crate::watch::Watch, fn(crate::watch::Error) -> Error>;

    /// A fixed volume's directory, descended by `path`, watched. A
    /// stored volume is [`Error::Unwatched`]: the container's proxy
    /// watches it, and on the hosts with a machine its loop mount is
    /// the machine's, which this host cannot watch.
    async fn watch(&self, path: &[String]) -> Result<Self::Watch, Error> {
        let Place::Fixed { root, .. } = &self.inner.place else {
            return Err(Error::Unwatched(self.inner.name.clone()));
        };
        let mut dir = root.clone();
        dir.extend(path);
        Ok(crate::watch::watch(&dir).await?.map_err(Error::Watch as fn(crate::watch::Error) -> Error))
    }

    /// The file's pieces, as a channel a reader fills from the
    /// blocking pool or from `tokio::fs`, boxed: one shape for a
    /// stored volume's image and a fixed volume's directory.
    type Read = BoxStream<'static, Result<Bytes, Error>>;

    /// The file at `path` read out: from a stored volume's image
    /// through `fstool`, on the blocking pool, and from a fixed
    /// volume's directory through `tokio::fs`, a link followed by the
    /// OS. Either way a bounded channel carries the pieces, so the
    /// reader runs no further ahead than the caller takes; a file
    /// that cannot be opened is the error here, and a read that dies
    /// partway the last item.
    async fn read(&self, path: &[String]) -> Result<Self::Read, Error> {
        let (sender, receiver) = mpsc::channel(2);
        match &self.inner.place {
            Place::Stored { image, .. } => {
                let image = image.clone();
                let path = path.to_vec();
                tokio::spawn(async move { image::read_file(&image, &path, sender).await });
            }
            Place::Fixed { root, .. } => {
                let mut file = root.clone();
                file.extend(path);
                let opened = fs::File::open(&file).await;
                let meta = fs::metadata(&file).await;
                let file = match (opened, meta) {
                    (Err(error), _) if error.kind() == std::io::ErrorKind::NotFound => {
                        return Err(Error::Missing(path.to_vec()));
                    }
                    (Err(error), _) => return Err(Error::Io(error)),
                    (Ok(_), Ok(meta)) if !meta.is_file() => return Err(Error::NotAFile(path.to_vec())),
                    (Ok(file), _) => file,
                };
                tokio::spawn(read_host(file, sender));
            }
        }
        Ok(receiver_stream(receiver).boxed())
    }

    /// The file at `path` written in, whole, from `content`: into a
    /// stored volume's image through `fstool`, on the blocking pool,
    /// and into a fixed volume's directory through `tokio::fs` — every
    /// missing parent made, the content into a temporary beside the
    /// destination, the temporary moved over. Whatever a walk found is
    /// forgotten first: the content is about to change.
    async fn write<S>(&self, path: &[String], content: S) -> Result<(), Error>
    where
        S: Stream<Item = Result<Bytes, error::Error>> + Send + 'static,
    {
        *self.inner.walked.lock().await = None;
        match &self.inner.place {
            Place::Stored { image, .. } => {
                let (sender, receiver) = mpsc::channel(2);
                let forward = tokio::spawn(forward(content, sender));
                let written = image::write_file(image, path, receiver).await;
                forward.abort();
                written
            }
            Place::Fixed { root, .. } => write_host(root, path, content).await,
        }
    }

    /// The subtree at `path` as a snapshot's nodes: a stored volume's
    /// image walked through `fstool`, a fixed volume's directory
    /// walked by the watch's own walker with every directory dark, so
    /// `changes` is `false` throughout.
    async fn filetree(&self, path: &[String]) -> Result<Vec<Node>, Error> {
        match &self.inner.place {
            Place::Stored { image, .. } => image::tree(image, path).await,
            Place::Fixed { root, .. } => {
                let mut dir = root.clone();
                dir.extend(path);
                let root = root.clone();
                let meta = fs::metadata(&dir).await;
                match meta {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        return Err(Error::Missing(path.to_vec()));
                    }
                    Err(error) => return Err(Error::Io(error)),
                    Ok(meta) if !meta.is_dir() => return Err(Error::NotADirectory(path.to_vec())),
                    Ok(_) => {}
                }
                tokio::task::spawn_blocking(move || crate::watch::children(&dir, &root, &[root.clone()]))
                    .await
                    .map_err(|error| Error::Io(std::io::Error::other(error)))
            }
        }
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

    /// The volume changed: its image resized, with the filesystem in
    /// it; its mode file rewritten; or the one and then the other.
    ///
    /// Under the SDK's exclusive hold, so no container has the image.
    /// A fixed volume is refused whatever the change. A change of the
    /// size is [`resize`](Self::resize); a change of the mode writes
    /// the mode file and then what this holds, so a reader never sees
    /// a mode the disk does not; a change of both resizes first and
    /// changes the mode only on `Edited`, so a size refused leaves
    /// the mode as it was and the answer means the volume is as it
    /// was in every respect.
    async fn edit(&self, change: Change) -> Result<Edit, Error> {
        if let Place::Fixed { .. } = &self.inner.place {
            return Err(Error::Fixed(self.inner.name.clone()));
        }
        match change {
            Change::Bytes(bytes) => self.resize(bytes).await,
            Change::Persist(persist) => {
                self.set_persist(persist).await?;
                Ok(Edit::Edited)
            }
            Change::Both { bytes, persist } => {
                let edit = self.resize(bytes).await?;
                if edit == Edit::Edited {
                    self.set_persist(persist).await?;
                }
                Ok(edit)
            }
        }
    }
}

impl Volume {
    /// The mode file rewritten and then what this holds, in that
    /// order: a failure to write leaves both as they were.
    async fn set_persist(&self, persist: bool) -> Result<(), Error> {
        let Place::Stored { image, .. } = &self.inner.place else {
            return Err(Error::Fixed(self.inner.name.clone()));
        };
        let path = mode_file(image);
        mode::write_mode(&path, Mode { persist }).await?;
        self.inner.persist.store(persist, Ordering::Release);
        Ok(())
    }

    /// The image resized, with the filesystem in it.
    ///
    /// A size the volume could not have been created at is refused —
    /// under 4 MiB, or over what the formatter addresses. The size it
    /// has already is answered `Edited` with nothing run. Smaller is
    /// a shrink: refused as `ContentTooLarge` when the walk's
    /// `bytes_used` exceeds the size, else the filesystem shrunk and
    /// then the file. Larger is a grow: refused as
    /// `InsufficientCapacity` when the store lacks the difference,
    /// else the file lengthened and then the filesystem. Either way
    /// the tools are the system's `e2fsck` and `resize2fs` — see
    /// [`resize`] for where — and a content that fits the size but
    /// whose metadata does not is their refusal, returned as the
    /// error, with the volume as it was. A resize changes no file's
    /// content, so what a walk found stands.
    async fn resize(&self, bytes: u64) -> Result<Edit, Error> {
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
