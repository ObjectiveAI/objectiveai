//! A volume, served: the SDK's `Served`, answering the FUSE asks from
//! a stored volume's image or a fixed volume's directory.
//!
//! A stored volume is opened once, when the serve begins — the image
//! as fstool's device and filesystem, the journal replayed — and every
//! ask runs against that one filesystem on the blocking pool, one at a
//! time, since the filesystem is `&mut`; a fixed volume answers from
//! its directory through `tokio::fs`. Nothing is buffered: a read is
//! one piece at an offset, a write lands one piece in place, and the
//! filesystem's metadata is flushed after every change.
//!
//! The volume's mode is kept here. A persistent volume's image is
//! opened read-write, this serve being its one user. A read-only
//! volume's image is opened read-only, and every mutation answers
//! `ReadOnly` before touching it. An ephemeral volume's image is
//! opened under an [`Overlay`]: reads come from the image, writes
//! land in a scratch file of this serve's own that goes with it, and
//! a write that would take the scratch past the serve's
//! `overlay_disk` is refused for that ask alone. A mutation that
//! fails part-way — the cap, or anything else — is followed by the
//! filesystem being opened again over the same device, so nothing it
//! had staged in memory outlives the failure.

use std::io::{self, Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bytes::Bytes;
use diverge_provider_sdk::endpoints::volumes::Mode;
use diverge_provider_sdk::server::served;
use diverge_provider_sdk::shared::containers::fuse::ack::Refused;
use diverge_provider_sdk::shared::containers::fuse::stat::Stat;
use diverge_provider_sdk::shared::containers::fuse::{Attrs, Kind, Listed, Time};
use fstool::block::{BlockDevice, FileBackend};
use fstool::fs::ext::Ext;
use fstool::fs::{EntryKind, FileMeta, Filesystem as _, OpenFlags, SetAttrs};
use tokio::sync::Mutex;

use super::{Error, Overlay, Scratch};

/// A volume being served: where its content is, and its mode.
pub enum Served {
    /// A stored volume's image, open for the serve's life.
    Stored {
        /// The device and the filesystem in it, one at a time.
        opened: Arc<Mutex<Opened>>,
        mode: Mode,
    },
    /// A fixed volume's directory.
    Fixed { root: PathBuf, mode: Mode },
}

/// An image, open: fstool's device — the image, read-write or
/// read-only, or the image under an overlay — and the ext4
/// filesystem it holds.
pub struct Opened {
    device: Box<dyn BlockDevice>,
    filesystem: Ext,
}

impl Opened {
    /// The filesystem opened again over the same device, the journal
    /// replayed: what a failed change had staged is dropped.
    fn reopen(&mut self) -> Result<(), fstool::Error> {
        let mut filesystem = Ext::open(&mut *self.device)?;
        filesystem.replay_pending_journal(&mut *self.device)?;
        self.filesystem = filesystem;
        Ok(())
    }
}

impl Served {
    /// A stored volume's image opened for serving, on the blocking
    /// pool, as its mode says: read-write, read-only, or under an
    /// overlay of `overlay_disk` bytes taken from `scratch`'s cap —
    /// [`Error::OverlayDisk`] when the cap has no room. The journal is
    /// replayed first, as a walk replays it; on a read-only image a
    /// replay the device refuses is left, the filesystem being as of
    /// its last checkpoint.
    pub async fn stored(image: &Path, mode: Mode, overlay_disk: u64, scratch: &Scratch) -> Result<Self, Error> {
        let image = image.to_path_buf();
        let overlay = match mode {
            Mode::Ephemeral => {
                let lease = scratch.lease(overlay_disk).ok_or(Error::OverlayDisk(overlay_disk))?;
                Some((scratch.create().await?, lease))
            }
            Mode::Persistent | Mode::ReadOnly => None,
        };
        let opened = tokio::task::spawn_blocking(move || {
            let mut device: Box<dyn BlockDevice> = match (mode, overlay) {
                (Mode::Ephemeral, Some((file, lease))) => Box::new(Overlay::open(&image, file, overlay_disk, lease)?),
                (Mode::ReadOnly, _) => Box::new(FileBackend::open_read_only(&image)?),
                (Mode::Persistent | Mode::Ephemeral, _) => Box::new(FileBackend::open(&image)?),
            };
            let mut filesystem = Ext::open(&mut *device)?;
            match filesystem.replay_pending_journal(&mut *device) {
                Ok(_) => {}
                Err(fstool::Error::Io(error)) if mode == Mode::ReadOnly && error.kind() == io::ErrorKind::PermissionDenied => {}
                Err(error) => return Err(Error::Format(error)),
            }
            Ok::<Opened, Error>(Opened { device, filesystem })
        })
        .await
        .map_err(io::Error::other)??;
        Ok(Served::Stored {
            opened: Arc::new(Mutex::new(opened)),
            mode,
        })
    }

    /// A fixed volume's directory, served as it is.
    pub fn fixed(root: &Path, mode: Mode) -> Self {
        Served::Fixed {
            root: root.to_path_buf(),
            mode,
        }
    }

    fn read_only(&self) -> bool {
        match self {
            Served::Stored { mode, .. } | Served::Fixed { mode, .. } => *mode == Mode::ReadOnly,
        }
    }

    /// `f` run on the open image, on the blocking pool, the image held
    /// for its duration.
    async fn image<T: Send + 'static>(
        opened: &Arc<Mutex<Opened>>,
        f: impl FnOnce(&mut Ext, &mut dyn BlockDevice) -> Result<T, fstool::Error> + Send + 'static,
    ) -> Result<T, String> {
        let opened = Arc::clone(opened);
        tokio::task::spawn_blocking(move || {
            let mut guard = opened.blocking_lock();
            let Opened { device, filesystem } = &mut *guard;
            f(filesystem, &mut **device).map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())?
    }

    /// `f` run on the open image as a change: on a failure the
    /// filesystem is opened again over the device, so what the change
    /// staged before it failed is dropped rather than flushed later.
    async fn mutate(
        opened: &Arc<Mutex<Opened>>,
        f: impl FnOnce(&mut Ext, &mut dyn BlockDevice) -> Result<(), fstool::Error> + Send + 'static,
    ) -> Result<(), String> {
        let opened = Arc::clone(opened);
        tokio::task::spawn_blocking(move || {
            let mut guard = opened.blocking_lock();
            let Opened { device, filesystem } = &mut *guard;
            match f(filesystem, &mut **device) {
                Ok(()) => Ok(()),
                Err(error) => {
                    let message = match guard.reopen() {
                        Ok(()) => error.to_string(),
                        Err(reopen) => format!("{error}; and the filesystem could not be reopened: {reopen}"),
                    };
                    Err(message)
                }
            }
        })
        .await
        .map_err(|error| error.to_string())?
    }
}

/// A path as the image spells one: `/`-joined from the root.
fn inside(path: &str) -> String {
    format!("/{path}")
}

/// Whether an fstool error is nothing at the path.
fn missing(error: &fstool::Error) -> bool {
    matches!(error, fstool::Error::InvalidArgument(_))
}

/// A wire time from whole seconds.
fn seconds(secs: u32) -> Time {
    Time {
        secs: u64::from(secs),
        nanos: 0,
    }
}

impl served::Served for Served {
    async fn stat(&self, path: &str) -> Result<Option<Stat>, String> {
        match self {
            Served::Stored { opened, .. } => {
                let path = inside(path);
                Self::image(opened, move |filesystem, device| match filesystem.getattr(device, Path::new(&path)) {
                    Ok(attrs) => Ok(Some(Stat {
                        kind: if attrs.kind == EntryKind::Dir { Kind::Directory } else { Kind::File },
                        size: if attrs.kind == EntryKind::Dir { 0 } else { attrs.size },
                        mode: u32::from(attrs.mode) & 0o7777,
                        uid: attrs.uid,
                        gid: attrs.gid,
                        atime: seconds(attrs.atime),
                        mtime: seconds(attrs.mtime),
                        ctime: seconds(attrs.ctime),
                    })),
                    Err(error) if missing(&error) => Ok(None),
                    Err(error) => Err(error),
                })
                .await
            }
            Served::Fixed { root, .. } => host::stat(&root.join(path)).await,
        }
    }

    async fn read(&self, path: &str, offset: u64, length: u32) -> Result<Option<Bytes>, String> {
        match self {
            Served::Stored { opened, .. } => {
                let path = inside(path);
                Self::image(opened, move |filesystem, device| {
                    let mut handle = match filesystem.open_file_ro(device, Path::new(&path)) {
                        Ok(handle) => handle,
                        Err(error) if missing(&error) => return Ok(None),
                        Err(error) => return Err(error),
                    };
                    if offset >= handle.len() {
                        return Ok(Some(Bytes::new()));
                    }
                    handle.seek(SeekFrom::Start(offset))?;
                    let want = usize::try_from(u64::from(length).min(handle.len() - offset)).unwrap_or(usize::MAX);
                    let mut buffer = vec![0u8; want];
                    let mut filled = 0;
                    while filled < want {
                        let n = handle.read(&mut buffer[filled..])?;
                        if n == 0 {
                            break;
                        }
                        filled += n;
                    }
                    buffer.truncate(filled);
                    Ok(Some(Bytes::from(buffer)))
                })
                .await
            }
            Served::Fixed { root, .. } => host::read(&root.join(path), offset, length).await,
        }
    }

    async fn write(&self, path: &str, offset: u64, bytes: Bytes) -> Result<(), Refused> {
        if self.read_only() {
            return Err(Refused::ReadOnly);
        }
        match self {
            Served::Stored { opened, .. } => {
                let path = inside(path);
                Self::mutate(opened, move |filesystem, device| {
                    let mut handle = filesystem.open_file_rw(
                        device,
                        Path::new(&path),
                        OpenFlags {
                            create: true,
                            truncate: false,
                            append: false,
                        },
                        Some(FileMeta::default()),
                    )?;
                    if handle.len() < offset {
                        handle.set_len(offset)?;
                    }
                    handle.seek(SeekFrom::Start(offset))?;
                    handle.write_all(&bytes)?;
                    handle.sync()?;
                    drop(handle);
                    filesystem.flush(device)
                })
                .await
                .map_err(Refused::Error)
            }
            Served::Fixed { root, .. } => host::write(&root.join(path), offset, &bytes).await,
        }
    }

    async fn truncate(&self, path: &str, size: u64) -> Result<(), Refused> {
        if self.read_only() {
            return Err(Refused::ReadOnly);
        }
        match self {
            Served::Stored { opened, .. } => {
                let path = inside(path);
                Self::mutate(opened, move |filesystem, device| {
                    let mut handle = filesystem.open_file_rw(
                        device,
                        Path::new(&path),
                        OpenFlags {
                            create: true,
                            truncate: false,
                            append: false,
                        },
                        Some(FileMeta::default()),
                    )?;
                    handle.set_len(size)?;
                    handle.sync()?;
                    drop(handle);
                    filesystem.flush(device)
                })
                .await
                .map_err(Refused::Error)
            }
            Served::Fixed { root, .. } => host::truncate(&root.join(path), size).await,
        }
    }

    async fn setattr(&self, path: &str, attrs: Attrs) -> Result<(), Refused> {
        if self.read_only() {
            return Err(Refused::ReadOnly);
        }
        match self {
            Served::Stored { opened, .. } => {
                let path = inside(path);
                Self::mutate(opened, move |filesystem, device| {
                    let set = SetAttrs {
                        mode: attrs.mode.map(|mode| (mode & 0o7777) as u16),
                        uid: attrs.uid,
                        gid: attrs.gid,
                        atime: attrs.atime.map(|time| u32::try_from(time.secs).unwrap_or(u32::MAX)),
                        mtime: attrs.mtime.map(|time| u32::try_from(time.secs).unwrap_or(u32::MAX)),
                        ctime: None,
                    };
                    filesystem.set_attrs(device, Path::new(&path), set)?;
                    filesystem.flush(device)
                })
                .await
                .map_err(Refused::Error)
            }
            Served::Fixed { root, .. } => host::setattr(&root.join(path), attrs).await,
        }
    }

    async fn list(&self, path: &str) -> Result<Option<Vec<Listed>>, String> {
        match self {
            Served::Stored { opened, .. } => {
                let path = inside(path);
                Self::image(opened, move |filesystem, device| {
                    match filesystem.getattr(device, Path::new(&path)) {
                        Ok(attrs) if attrs.kind == EntryKind::Dir => {}
                        Ok(_) => return Ok(None),
                        Err(error) if missing(&error) => return Ok(None),
                        Err(error) => return Err(error),
                    }
                    let entries = filesystem.list(device, Path::new(&path))?;
                    Ok(Some(
                        entries
                            .into_iter()
                            .filter(|entry| entry.name != "." && entry.name != "..")
                            .map(|entry| Listed {
                                kind: if entry.kind == EntryKind::Dir { Kind::Directory } else { Kind::File },
                                name: entry.name,
                            })
                            .collect(),
                    ))
                })
                .await
            }
            Served::Fixed { root, .. } => host::list(&root.join(path)).await,
        }
    }

    async fn remove(&self, path: &str) -> Result<(), Refused> {
        if self.read_only() {
            return Err(Refused::ReadOnly);
        }
        match self {
            Served::Stored { opened, .. } => {
                let path = inside(path);
                Self::mutate(opened, move |filesystem, device| {
                    filesystem.remove(device, Path::new(&path))?;
                    filesystem.flush(device)
                })
                .await
                .map_err(Refused::Error)
            }
            Served::Fixed { root, .. } => host::remove(&root.join(path)).await,
        }
    }

    async fn rename(&self, from: &str, to: &str) -> Result<(), Refused> {
        if self.read_only() {
            return Err(Refused::ReadOnly);
        }
        match self {
            Served::Stored { opened, .. } => {
                let (from, to) = (inside(from), inside(to));
                Self::mutate(opened, move |filesystem, device| {
                    // A file at the destination is replaced, as a rename
                    // replaces one; the trait's rename, by name, since
                    // `Ext` has an inherent one of another shape.
                    match filesystem.getattr(device, Path::new(&to)) {
                        Ok(attrs) if attrs.kind != EntryKind::Dir => filesystem.remove(device, Path::new(&to))?,
                        _ => {}
                    }
                    fstool::fs::Filesystem::rename(filesystem, device, Path::new(&from), Path::new(&to))?;
                    filesystem.flush(device)
                })
                .await
                .map_err(Refused::Error)
            }
            Served::Fixed { root, .. } => host::rename(&root.join(from), &root.join(to)).await,
        }
    }

    async fn mkdir(&self, path: &str) -> Result<(), Refused> {
        if self.read_only() {
            return Err(Refused::ReadOnly);
        }
        match self {
            Served::Stored { opened, .. } => {
                let path = inside(path);
                Self::mutate(opened, move |filesystem, device| {
                    filesystem.create_dir(
                        device,
                        Path::new(&path),
                        FileMeta {
                            mode: 0o755,
                            ..FileMeta::default()
                        },
                    )?;
                    filesystem.flush(device)
                })
                .await
                .map_err(Refused::Error)
            }
            Served::Fixed { root, .. } => host::mkdir(&root.join(path)).await,
        }
    }
}

/// The asks answered from a fixed volume's directory, through the
/// host's own filesystem. Mode, owner and group are the host's where
/// the host has them, and `0o644`, `0o755` and root where it does not.
mod host {
    use std::io::SeekFrom;
    use std::path::Path;

    use bytes::Bytes;
    use diverge_provider_sdk::shared::containers::fuse::ack::Refused;
    use diverge_provider_sdk::shared::containers::fuse::stat::Stat;
    use diverge_provider_sdk::shared::containers::fuse::{Attrs, Kind, Listed, Time};
    use tokio::fs;
    use tokio::io::{AsyncReadExt as _, AsyncSeekExt as _, AsyncWriteExt as _};

    /// A system time as the wire's; before 1970 is 1970.
    fn time(time: std::io::Result<std::time::SystemTime>) -> Time {
        let since = time
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .unwrap_or_default();
        Time {
            secs: since.as_secs(),
            nanos: since.subsec_nanos(),
        }
    }

    /// The mode, the owner and the group, as the host records them.
    #[cfg(unix)]
    fn owner(meta: &std::fs::Metadata) -> (u32, u32, u32) {
        use std::os::unix::fs::MetadataExt as _;
        (meta.mode() & 0o7777, meta.uid(), meta.gid())
    }

    /// The mode, the owner and the group where the host records none.
    #[cfg(not(unix))]
    fn owner(meta: &std::fs::Metadata) -> (u32, u32, u32) {
        let mut mode = if meta.is_dir() { 0o755 } else { 0o644 };
        if meta.permissions().readonly() {
            mode &= !0o222;
        }
        (mode, 0, 0)
    }

    fn io(error: std::io::Error) -> String {
        error.to_string()
    }

    fn refused(error: std::io::Error) -> Refused {
        Refused::Error(error.to_string())
    }

    pub async fn stat(path: &Path) -> Result<Option<Stat>, String> {
        let meta = match fs::metadata(path).await {
            Ok(meta) => meta,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(io(error)),
        };
        let (mode, uid, gid) = owner(&meta);
        let modified = time(meta.modified());
        Ok(Some(Stat {
            kind: if meta.is_dir() { Kind::Directory } else { Kind::File },
            size: if meta.is_dir() { 0 } else { meta.len() },
            mode,
            uid,
            gid,
            atime: time(meta.accessed()),
            mtime: modified,
            ctime: modified,
        }))
    }

    pub async fn read(path: &Path, offset: u64, length: u32) -> Result<Option<Bytes>, String> {
        let mut file = match fs::File::open(path).await {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(io(error)),
        };
        file.seek(SeekFrom::Start(offset)).await.map_err(io)?;
        let mut buffer = vec![0u8; length as usize];
        let mut filled = 0;
        while filled < buffer.len() {
            let n = file.read(&mut buffer[filled..]).await.map_err(io)?;
            if n == 0 {
                break;
            }
            filled += n;
        }
        buffer.truncate(filled);
        Ok(Some(Bytes::from(buffer)))
    }

    pub async fn write(path: &Path, offset: u64, bytes: &[u8]) -> Result<(), Refused> {
        let mut file = fs::OpenOptions::new().write(true).create(true).truncate(false).open(path).await.map_err(refused)?;
        file.seek(SeekFrom::Start(offset)).await.map_err(refused)?;
        file.write_all(bytes).await.map_err(refused)?;
        file.flush().await.map_err(refused)
    }

    pub async fn truncate(path: &Path, size: u64) -> Result<(), Refused> {
        let file = fs::OpenOptions::new().write(true).create(true).truncate(false).open(path).await.map_err(refused)?;
        file.set_len(size).await.map_err(refused)
    }

    pub async fn setattr(path: &Path, attrs: Attrs) -> Result<(), Refused> {
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt as _;
                if let Some(mode) = attrs.mode {
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode & 0o7777)).map_err(refused)?;
                }
                if attrs.uid.is_some() || attrs.gid.is_some() {
                    std::os::unix::fs::chown(&path, attrs.uid, attrs.gid).map_err(refused)?;
                }
            }
            if attrs.atime.is_some() || attrs.mtime.is_some() {
                let at = |time: Time| std::time::UNIX_EPOCH + std::time::Duration::new(time.secs, time.nanos);
                let mut times = std::fs::FileTimes::new();
                if let Some(atime) = attrs.atime {
                    times = times.set_accessed(at(atime));
                }
                if let Some(mtime) = attrs.mtime {
                    times = times.set_modified(at(mtime));
                }
                let file = std::fs::OpenOptions::new().write(true).open(&path).map_err(refused)?;
                file.set_times(times).map_err(refused)?;
            }
            Ok(())
        })
        .await
        .map_err(|error| Refused::Error(error.to_string()))?
    }

    pub async fn list(path: &Path) -> Result<Option<Vec<Listed>>, String> {
        let mut entries = match fs::read_dir(path).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound || error.kind() == std::io::ErrorKind::NotADirectory => return Ok(None),
            Err(error) => return Err(io(error)),
        };
        let mut listed = Vec::new();
        while let Some(entry) = entries.next_entry().await.map_err(io)? {
            let kind = entry.file_type().await.map_err(io)?;
            listed.push(Listed {
                name: entry.file_name().to_string_lossy().into_owned(),
                kind: if kind.is_dir() { Kind::Directory } else { Kind::File },
            });
        }
        Ok(Some(listed))
    }

    pub async fn remove(path: &Path) -> Result<(), Refused> {
        let meta = fs::metadata(path).await.map_err(refused)?;
        if meta.is_dir() {
            fs::remove_dir(path).await.map_err(refused)
        } else {
            fs::remove_file(path).await.map_err(refused)
        }
    }

    pub async fn rename(from: &Path, to: &Path) -> Result<(), Refused> {
        fs::rename(from, to).await.map_err(refused)
    }

    pub async fn mkdir(path: &Path) -> Result<(), Refused> {
        fs::create_dir(path).await.map_err(refused)
    }
}
