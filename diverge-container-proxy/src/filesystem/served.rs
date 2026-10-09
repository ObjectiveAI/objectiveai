//! A subtree of the container answered as a filesystem: the nine
//! asks, from the proxy's own disk.

use std::io::SeekFrom;
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use diverge_sdk::shared::containers::fuse::ack::Refused;
use diverge_sdk::shared::containers::fuse::stat::Stat;
use diverge_sdk::shared::containers::fuse::{Attrs, Kind, Listed, Time};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt as _, AsyncSeekExt as _, AsyncWriteExt as _};

use super::tree::Ignore;

/// A subtree served: its root on disk, and what is left out of it.
pub struct Served {
    root: PathBuf,
    ignore: Ignore,
}

impl Served {
    /// The subtree at `root`, less the proxy's own three — `/proc`,
    /// `/sys` and `/dev` — under which nothing is answered.
    pub fn new(root: PathBuf) -> Self {
        Served {
            root,
            ignore: Ignore::new(Vec::new()),
        }
    }

    /// Where the path of an ask is: the entry under the root, every
    /// component a name. `None` for a path that is not one of the
    /// subtree's — a `.` or `..` among its components, or a resolved
    /// path the proxy leaves out — and such a path holds nothing.
    fn resolve(&self, path: &str) -> Option<PathBuf> {
        let mut resolved = self.root.clone();
        for component in path.split('/').filter(|component| !component.is_empty()) {
            if component == "." || component == ".." {
                return None;
            }
            resolved.push(component);
        }
        if self.ignore.excluded(&resolved) {
            return None;
        }
        Some(resolved)
    }

    /// The resolved path, or the refusal every mutation gives a path
    /// that is not the subtree's.
    fn resolve_for_change(&self, path: &str) -> Result<PathBuf, Refused> {
        self.resolve(path).ok_or_else(|| Refused::Error("not a path of the serve".to_string()))
    }

    /// What is at the path: its kind, its size, and the mode, the
    /// owner, the group and the times as the filesystem records them.
    /// A symbolic link is the link itself, a file.
    pub async fn stat(&self, path: &str) -> Result<Option<Stat>, String> {
        let Some(resolved) = self.resolve(path) else {
            return Ok(None);
        };
        match tokio::fs::symlink_metadata(&resolved).await {
            Ok(meta) => {
                let directory = meta.is_dir();
                Ok(Some(Stat {
                    kind: if directory { Kind::Directory } else { Kind::File },
                    size: if directory { 0 } else { meta.len() },
                    mode: meta.mode() & 0o7777,
                    uid: meta.uid(),
                    gid: meta.gid(),
                    atime: time(meta.atime(), meta.atime_nsec()),
                    mtime: time(meta.mtime(), meta.mtime_nsec()),
                    ctime: time(meta.ctime(), meta.ctime_nsec()),
                }))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    /// A piece of the file at the path: at most `length` bytes from
    /// `offset`, fewer at the end, none at or past it.
    pub async fn read(&self, path: &str, offset: u64, length: u32) -> Result<Option<Bytes>, String> {
        let Some(resolved) = self.resolve(path) else {
            return Ok(None);
        };
        let mut file = match tokio::fs::File::open(&resolved).await {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        file.seek(SeekFrom::Start(offset)).await.map_err(|error| error.to_string())?;
        let mut buffer = vec![0u8; usize::try_from(length).unwrap_or(0)];
        let mut filled = 0;
        while filled < buffer.len() {
            let read = file.read(&mut buffer[filled..]).await.map_err(|error| error.to_string())?;
            if read == 0 {
                break;
            }
            filled += read;
        }
        buffer.truncate(filled);
        Ok(Some(Bytes::from(buffer)))
    }

    /// A piece written into the file at the path, in place: the file
    /// made if absent, extended with zeros to the offset if short.
    pub async fn write(&self, path: &str, offset: u64, bytes: &[u8]) -> Result<(), Refused> {
        let resolved = self.resolve_for_change(path)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&resolved)
            .await
            .map_err(|error| Refused::Error(error.to_string()))?;
        file.seek(SeekFrom::Start(offset)).await.map_err(|error| Refused::Error(error.to_string()))?;
        file.write_all(bytes).await.map_err(|error| Refused::Error(error.to_string()))?;
        file.flush().await.map_err(|error| Refused::Error(error.to_string()))
    }

    /// The file at the path made `size` long, cut or extended with
    /// zeros; made if absent.
    pub async fn truncate(&self, path: &str, size: u64) -> Result<(), Refused> {
        let resolved = self.resolve_for_change(path)?;
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&resolved)
            .await
            .map_err(|error| Refused::Error(error.to_string()))?;
        file.set_len(size).await.map_err(|error| Refused::Error(error.to_string()))
    }

    /// The attributes set that `attrs` sets, the rest left: the mode
    /// as permission bits, the owner and the group as the filesystem
    /// allows the proxy to set them, the access and modification
    /// times on the entry itself. On a blocking thread, since the
    /// times go through a handle `std` opens.
    pub async fn setattr(&self, path: &str, attrs: Attrs) -> Result<(), Refused> {
        let resolved = self.resolve_for_change(path)?;
        tokio::task::spawn_blocking(move || setattr_blocking(&resolved, attrs))
            .await
            .map_err(|error| Refused::Error(error.to_string()))?
            .map_err(|error| Refused::Error(error.to_string()))
    }

    /// The entries of the directory at the path, by name, those the
    /// proxy leaves out absent.
    pub async fn list(&self, path: &str) -> Result<Option<Vec<Listed>>, String> {
        let Some(resolved) = self.resolve(path) else {
            return Ok(None);
        };
        let mut entries = match tokio::fs::read_dir(&resolved).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        let mut listed = Vec::new();
        while let Some(entry) = entries.next_entry().await.map_err(|error| error.to_string())? {
            if self.ignore.excluded(&entry.path()) {
                continue;
            }
            let kind = match entry.file_type().await {
                Ok(file_type) if file_type.is_dir() => Kind::Directory,
                _ => Kind::File,
            };
            listed.push(Listed {
                name: entry.file_name().to_string_lossy().into_owned(),
                kind,
            });
        }
        listed.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
        Ok(Some(listed))
    }

    /// What is at the path removed: a file, or an empty directory; a
    /// directory with something in it is refused.
    pub async fn remove(&self, path: &str) -> Result<(), Refused> {
        let resolved = self.resolve_for_change(path)?;
        let meta = tokio::fs::symlink_metadata(&resolved)
            .await
            .map_err(|error| Refused::Error(error.to_string()))?;
        let removed = if meta.is_dir() {
            tokio::fs::remove_dir(&resolved).await
        } else {
            tokio::fs::remove_file(&resolved).await
        };
        removed.map_err(|error| Refused::Error(error.to_string()))
    }

    /// What is at `from` moved to `to`, both within the subtree.
    pub async fn rename(&self, from: &str, to: &str) -> Result<(), Refused> {
        let from = self.resolve_for_change(from)?;
        let to = self.resolve_for_change(to)?;
        tokio::fs::rename(&from, &to)
            .await
            .map_err(|error| Refused::Error(error.to_string()))
    }

    /// A directory made at the path, under one that exists.
    pub async fn mkdir(&self, path: &str) -> Result<(), Refused> {
        let resolved = self.resolve_for_change(path)?;
        tokio::fs::create_dir(&resolved)
            .await
            .map_err(|error| Refused::Error(error.to_string()))
    }
}

/// The attributes, set with `std`: permissions, ownership, times.
fn setattr_blocking(path: &Path, attrs: Attrs) -> std::io::Result<()> {
    if let Some(mode) = attrs.mode {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    }
    if attrs.uid.is_some() || attrs.gid.is_some() {
        std::os::unix::fs::chown(path, attrs.uid, attrs.gid)?;
    }
    if attrs.atime.is_some() || attrs.mtime.is_some() {
        let mut times = std::fs::FileTimes::new();
        if let Some(atime) = attrs.atime {
            times = times.set_accessed(system(atime));
        }
        if let Some(mtime) = attrs.mtime {
            times = times.set_modified(system(mtime));
        }
        std::fs::File::open(path)?.set_times(times)?;
    }
    Ok(())
}

/// Seconds and nanoseconds as the filesystem records them, as the
/// wire's time; a time before the epoch is the epoch.
fn time(secs: i64, nanos: i64) -> Time {
    Time {
        secs: u64::try_from(secs).unwrap_or(0),
        nanos: u32::try_from(nanos).unwrap_or(0),
    }
}

/// The wire's time as a system time.
fn system(time: Time) -> SystemTime {
    UNIX_EPOCH + Duration::new(time.secs, time.nanos)
}
