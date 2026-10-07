//! A resource's bytes answered as a filesystem: the daemon's own
//! content, in place or copied.

use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use diverge_sdk::provider::client::Listed;
use diverge_sdk::shared::containers::fuse::ack::Refused;
use diverge_sdk::shared::containers::fuse::stat::Stat;
use diverge_sdk::shared::containers::fuse::{Kind, Time};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt as _, AsyncSeekExt as _, AsyncWriteExt as _};

use super::Source;

/// A copy of `from` — a file, or a directory whole — at `to`, every
/// parent made: an ephemeral mount's own layer.
pub async fn copy(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(parent) = to.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|error| error.to_string())?;
    }
    let from = from.to_path_buf();
    let to = to.to_path_buf();
    tokio::task::spawn_blocking(move || copy_blocking(&from, &to))
        .await
        .map_err(|error| error.to_string())?
}

/// The copy, on a blocking thread.
fn copy_blocking(from: &Path, to: &Path) -> Result<(), String> {
    let meta = std::fs::metadata(from).map_err(|error| format!("{}: {error}", from.display()))?;
    if meta.is_file() {
        std::fs::copy(from, to).map_err(|error| format!("{}: {error}", from.display()))?;
        return Ok(());
    }
    std::fs::create_dir_all(to).map_err(|error| format!("{}: {error}", to.display()))?;
    for entry in std::fs::read_dir(from).map_err(|error| format!("{}: {error}", from.display()))? {
        let entry = entry.map_err(|error| format!("{}: {error}", from.display()))?;
        copy_blocking(&entry.path(), &to.join(entry.file_name()))?;
    }
    Ok(())
}

/// Where the path of an ask is, for the source: the file itself for a
/// file mount, whose path is empty; the entry under the root for a
/// directory mount, every component a name. None for a path that is
/// not one of the source's.
fn resolve(source: &Source, path: &str) -> Option<PathBuf> {
    match source {
        Source::File { path: file, .. } => path.is_empty().then(|| file.clone()),
        Source::Directory { root, .. } => {
            let mut resolved = root.clone();
            for component in path.split('/').filter(|component| !component.is_empty()) {
                if component == "." || component == ".." {
                    return None;
                }
                resolved.push(component);
            }
            Some(resolved)
        }
        Source::Volume { .. } => None,
    }
}

/// Whether the source refuses mutations.
fn read_only(source: &Source) -> bool {
    match source {
        Source::File { read_only, .. } | Source::Directory { read_only, .. } => *read_only,
        Source::Volume { .. } => false,
    }
}

/// What is at the path.
pub async fn stat(source: &Source, path: &str) -> Result<Option<Stat>, String> {
    let Some(resolved) = resolve(source, path) else {
        return Ok(None);
    };
    match tokio::fs::symlink_metadata(&resolved).await {
        Ok(meta) => {
            let directory = meta.is_dir();
            Ok(Some(Stat {
                kind: if directory { Kind::Directory } else { Kind::File },
                size: if directory { 0 } else { meta.len() },
                mode: if directory { 0o755 } else { 0o644 },
                uid: 0,
                gid: 0,
                atime: time(meta.accessed().ok()),
                mtime: time(meta.modified().ok()),
                ctime: time(meta.created().ok().or(meta.modified().ok())),
            }))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

/// A piece of the file at the path.
pub async fn read(source: &Source, path: &str, offset: u64, length: u32) -> Result<Option<Bytes>, String> {
    let Some(resolved) = resolve(source, path) else {
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

/// A piece written into the file at the path, made if absent.
pub async fn write(source: &Source, path: &str, offset: u64, bytes: &[u8]) -> Result<(), Refused> {
    if read_only(source) {
        return Err(Refused::ReadOnly);
    }
    let resolved = resolve(source, path).ok_or_else(|| Refused::Error("not a path of the mount".to_string()))?;
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

/// The file at the path made `size` long.
pub async fn truncate(source: &Source, path: &str, size: u64) -> Result<(), Refused> {
    if read_only(source) {
        return Err(Refused::ReadOnly);
    }
    let resolved = resolve(source, path).ok_or_else(|| Refused::Error("not a path of the mount".to_string()))?;
    let file = OpenOptions::new()
        .write(true)
        .open(&resolved)
        .await
        .map_err(|error| Refused::Error(error.to_string()))?;
    file.set_len(size).await.map_err(|error| Refused::Error(error.to_string()))
}

/// The attributes changed: a resource keeps no mode, owner or times
/// of its own, so there is nothing to change and nothing to refuse on
/// a mount that takes writes.
pub async fn setattr(source: &Source, path: &str) -> Result<(), Refused> {
    if read_only(source) {
        return Err(Refused::ReadOnly);
    }
    match resolve(source, path) {
        Some(_) => Ok(()),
        None => Err(Refused::Error("not a path of the mount".to_string())),
    }
}

/// The entries of the directory at the path.
pub async fn list(source: &Source, path: &str) -> Result<Option<Vec<Listed>>, String> {
    let Some(resolved) = resolve(source, path) else {
        return Ok(None);
    };
    let mut entries = match tokio::fs::read_dir(&resolved).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let mut listed = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(|error| error.to_string())? {
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

/// What is at the path removed: a file, or an empty directory.
pub async fn remove(source: &Source, path: &str) -> Result<(), Refused> {
    if read_only(source) {
        return Err(Refused::ReadOnly);
    }
    let resolved = resolve(source, path).ok_or_else(|| Refused::Error("not a path of the mount".to_string()))?;
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

/// What is at `from` moved to `to`.
pub async fn rename(source: &Source, from: &str, to: &str) -> Result<(), Refused> {
    if read_only(source) {
        return Err(Refused::ReadOnly);
    }
    let from = resolve(source, from).ok_or_else(|| Refused::Error("not a path of the mount".to_string()))?;
    let to = resolve(source, to).ok_or_else(|| Refused::Error("not a path of the mount".to_string()))?;
    tokio::fs::rename(&from, &to)
        .await
        .map_err(|error| Refused::Error(error.to_string()))
}

/// A directory made at the path.
pub async fn mkdir(source: &Source, path: &str) -> Result<(), Refused> {
    if read_only(source) {
        return Err(Refused::ReadOnly);
    }
    let resolved = resolve(source, path).ok_or_else(|| Refused::Error("not a path of the mount".to_string()))?;
    tokio::fs::create_dir(&resolved)
        .await
        .map_err(|error| Refused::Error(error.to_string()))
}

/// A system time as the wire's.
fn time(time: Option<SystemTime>) -> Time {
    time.and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|elapsed| Time {
            secs: elapsed.as_secs(),
            nanos: elapsed.subsec_nanos(),
        })
        .unwrap_or_default()
}
