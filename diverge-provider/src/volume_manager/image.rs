//! A stored volume's file: an ext4 filesystem in a sparse image, made
//! here, read here, and written here.
//!
//! The one place in the crate that is not `tokio::fs`: the formatter,
//! the reader and the writer are `fstool`'s, and its API is
//! synchronous over `&mut` — one filesystem, one device, one thread.
//! So each thing done to an image runs whole on the blocking pool,
//! sequentially by the library's design, and the calls here are the
//! `async` face of that: a walk, a file read out in pieces through a
//! channel, a file written in from pieces through one, a tree. What
//! touches the file as a file — making it, sizing it, removing it —
//! is `tokio::fs`.

use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use diverge_sdk::CHUNK_SIZE;
use diverge_sdk::shared::error;
use diverge_sdk::shared::filetree::response::Node;
use fstool::block::{BlockDevice, FileBackend};
use fstool::fs::ext::{Ext, FormatOpts, FsKind};
use fstool::fs::{EntryKind, FileMeta, Filesystem as _, OpenFlags};
use sha2::{Digest as _, Sha256};
use tokio::fs::{self, OpenOptions};
use tokio::sync::mpsc;

use super::{Error, Line, Walked, sparse, walk};

/// Reserve the image at `path`, `bytes` long: the file made, marked
/// sparse where that takes telling, and set to its length. The bytes
/// are reserved, not taken.
///
/// What a create does under the manager's reservation lock, so two
/// creates cannot both fit in the room one of them takes; the
/// [`format_image`] that follows is out from under it. A path that exists
/// already is an error, whatever is there.
pub async fn reserve_image(path: &Path, bytes: u64) -> Result<(), Error> {
    let file = OpenOptions::new().write(true).create_new(true).open(path).await?;
    let made = size(&file, bytes).await;
    drop(file);
    if let Err(error) = made {
        let _ = fs::remove_file(path).await;
        return Err(error);
    }
    Ok(())
}

/// The reserved file marked sparse and sized, in that order: a dense
/// file sized first would have taken its bytes.
async fn size(file: &fs::File, bytes: u64) -> Result<(), Error> {
    sparse::sparse(file).await?;
    file.set_len(bytes).await?;
    Ok(())
}

/// Format the reserved image at `path` as an empty ext4 filesystem
/// filling its `bytes`. A failure removes the file: a half-formatted
/// image is not a volume.
///
/// 4 KiB blocks, the size every kernel defaults to; one inode per
/// 16 KiB, `mke2fs`'s own ratio; a journal, `lost+found`, and the
/// formatter told the file reads back as zero already, since a fresh
/// sparse file does and writing the zeros would take the space the
/// sparseness saved. The UUID is derived from the path and the
/// moment, so two volumes never share one.
pub async fn format_image(path: &Path, bytes: u64) -> Result<(), Error> {
    let opts = FormatOpts {
        kind: FsKind::Ext4,
        // 4 KiB.
        block_size: 4096,
        blocks_count: u32::try_from(bytes / 4096).map_err(|_| Error::TooLarge(bytes))?,
        // 16 KiB per inode.
        inodes_count: u32::try_from(bytes / 16384).map_err(|_| Error::TooLarge(bytes))?,
        uuid: uuid(path),
        create_lost_found: true,
        prezeroed: true,
        ..FormatOpts::default()
    };
    let image = path.to_path_buf();
    let formatted = tokio::task::spawn_blocking(move || {
        let mut device = FileBackend::open(&image)?;
        Ext::format_with(&mut device, &opts)?;
        Ok::<(), Error>(())
    })
    .await
    .map_err(io::Error::other)?;
    if let Err(error) = formatted {
        let _ = fs::remove_file(path).await;
        return Err(error);
    }
    Ok(())
}

/// A UUID for a new filesystem: the first sixteen bytes of the
/// SHA-256 of the image's path and the nanoseconds of now, stamped
/// as a version-4 UUID so anything that reads it sees a well-formed
/// one.
fn uuid(path: &Path) -> [u8; 16] {
    let mut digest = Sha256::new();
    digest.update(path.to_string_lossy().as_bytes());
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_nanos()).unwrap_or(0);
    digest.update(now.to_le_bytes());
    let hash = digest.finalize();
    let mut uuid = [0u8; 16];
    uuid.copy_from_slice(&hash[..16]);
    uuid[6] = (uuid[6] & 0x0f) | 0x40;
    uuid[8] = (uuid[8] & 0x3f) | 0x80;
    uuid
}

/// Walk the image at `path` and hash its content, as
/// [`walk::walk_directory`] hashes a directory: the same files, the same
/// names, the same lines, the same `h1:` string, only read out of
/// the ext4 filesystem inside the image rather than the host's.
///
/// The image is opened for writing so a journal the last container
/// left pending is replayed first; the SDK's lock is held, so nothing
/// else has the image. Every entry of every directory is walked in
/// turn — the reader is one `&mut` filesystem, so nothing here runs
/// beside anything else — and the lines are sorted and hashed by
/// `walk::digest`. A symbolic link is opened through, inside the
/// image: a link to a directory, a link to nothing, and a chain of
/// more than forty links are the walk's error, as a device, a pipe
/// or a socket is, since Go could not read one either.
pub async fn walk_image(path: &Path) -> Result<Walked, Error> {
    let image = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut device = FileBackend::open(&image)?;
        let mut filesystem = Ext::open(&mut device)?;
        filesystem.replay_pending_journal(&mut device)?;
        let mut lines = Vec::new();
        walk_dir(&mut filesystem, &mut device, &[], &mut lines)?;
        Ok(walk::digest(lines))
    })
    .await
    .map_err(io::Error::other)?
}

/// One directory of the image, at `components` from the root: every
/// entry listed, and each walked or hashed in turn.
fn walk_dir(filesystem: &mut Ext, device: &mut dyn BlockDevice, components: &[String], lines: &mut Vec<Line>) -> Result<(), Error> {
    let entries = filesystem.list(device, Path::new(&inside(components)))?;
    for entry in entries {
        if entry.name == "." || entry.name == ".." {
            continue;
        }
        let mut below = components.to_vec();
        below.push(entry.name);
        match entry.kind {
            EntryKind::Dir => walk_dir(filesystem, device, &below, lines)?,
            EntryKind::Regular => lines.push(hash_file(filesystem, device, &below, below.join("/"))?),
            EntryKind::Symlink => {
                let target = resolve(filesystem, device, &below)?;
                lines.push(hash_file(filesystem, device, &target, below.join("/"))?);
            }
            EntryKind::Char | EntryKind::Block | EntryKind::Fifo | EntryKind::Socket | EntryKind::Unknown => {
                return Err(Error::Io(io::Error::other(format!(
                    "dirhash: {} is not a regular file",
                    below.join("/")
                ))));
            }
        }
    }
    Ok(())
}

/// Where a symbolic link at `components` ends up: the regular file it
/// opens to, through any links on the way, as components from the
/// root. A target outside the root is clamped at the root, as the
/// kernel clamps `..` at a mount's root.
fn resolve(filesystem: &mut Ext, device: &mut dyn BlockDevice, components: &[String]) -> Result<Vec<String>, Error> {
    let mut at = components.to_vec();
    // Forty: the kernel's own limit on a chain of links.
    for _ in 0..40 {
        let target = filesystem.read_symlink(device, Path::new(&inside(&at)))?;
        let target = target.to_string_lossy();
        let mut resolved: Vec<String> = if target.starts_with('/') {
            Vec::new()
        } else {
            at[..at.len() - 1].to_vec()
        };
        for component in target.split('/') {
            match component {
                "" | "." => {}
                ".." => {
                    resolved.pop();
                }
                name => resolved.push(name.to_string()),
            }
        }
        match filesystem.getattr(device, Path::new(&inside(&resolved)))?.kind {
            EntryKind::Regular => return Ok(resolved),
            EntryKind::Symlink => at = resolved,
            _ => {
                return Err(Error::Io(io::Error::other(format!(
                    "dirhash: {} does not open to a regular file",
                    components.join("/")
                ))));
            }
        }
    }
    Err(Error::Io(io::Error::other(format!(
        "dirhash: too many levels of symbolic links at {}",
        components.join("/")
    ))))
}

/// One file of the image, at `components` from the root, hashed under
/// `name`: the SHA-256 of its bytes as lowercase hexadecimal, and its
/// length, read in chunks. A name containing a newline is refused, as
/// Go refuses it.
fn hash_file(filesystem: &mut Ext, device: &mut dyn BlockDevice, components: &[String], name: String) -> Result<Line, Error> {
    if name.contains('\n') {
        return Err(Error::Io(walk::newline(&name)));
    }
    let mut reader = filesystem.read_file(device, Path::new(&inside(components)))?;
    let mut hasher = Sha256::new();
    // 64 KiB at a time.
    let mut buffer = vec![0u8; 64 * 1024];
    let mut size = 0u64;
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        size += n as u64;
    }
    Ok(Line {
        name,
        hex: hex::encode(hasher.finalize()),
        size,
    })
}

/// Read the file at `components` out of the image at `path`, in
/// pieces of at most [`CHUNK_SIZE`] sent down `sender`, until the file
/// is read or the receiver is gone.
///
/// The whole read runs on the blocking pool — the reader is one
/// `&mut` filesystem — and the sender is what makes it a stream: a
/// bounded channel, so the reader holds no more than the receiver
/// has not yet taken. The image is opened for writing so a pending
/// journal is replayed first, as a walk does; the SDK's lock is held,
/// so nothing else has the image. What is at the path is resolved
/// through any links, as a walk resolves one, and must be a regular
/// file: nothing there is [`Error::Missing`], anything else
/// [`Error::NotAFile`]. A zero-byte file sends one empty piece. A
/// failure is sent last, and the pieces before it stand.
pub async fn read_file(path: &Path, components: &[String], sender: mpsc::Sender<Result<Bytes, Error>>) {
    let image = path.to_path_buf();
    let components = components.to_vec();
    let pieces = sender.clone();
    let read = tokio::task::spawn_blocking(move || {
        let mut device = FileBackend::open(&image)?;
        let mut filesystem = Ext::open(&mut device)?;
        filesystem.replay_pending_journal(&mut device)?;
        let file = regular(&mut filesystem, &mut device, &components)?;
        let mut reader = filesystem.read_file(&mut device, Path::new(&inside(&file)))?;
        let mut buffer = vec![0u8; CHUNK_SIZE];
        let mut sent = 0u64;
        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 && sent > 0 {
                break;
            }
            sent += n as u64;
            if pieces.blocking_send(Ok(Bytes::copy_from_slice(&buffer[..n]))).is_err() {
                break;
            }
            if n == 0 {
                break;
            }
        }
        Ok::<(), Error>(())
    })
    .await
    .map_err(io::Error::other)
    .map_err(Error::Io)
    .and_then(|read| read);
    if let Err(error) = read {
        let _ = sender.send(Err(error)).await;
    }
}

/// What is at `components`, resolved through any links to a regular
/// file, as components from the root: nothing there is
/// [`Error::Missing`], anything but a regular file
/// [`Error::NotAFile`].
fn regular(filesystem: &mut Ext, device: &mut dyn BlockDevice, components: &[String]) -> Result<Vec<String>, Error> {
    let attrs = match filesystem.getattr(device, Path::new(&inside(components))) {
        Ok(attrs) => attrs,
        Err(fstool::Error::InvalidArgument(_)) => return Err(Error::Missing(components.to_vec())),
        Err(error) => return Err(Error::Format(error)),
    };
    match attrs.kind {
        EntryKind::Regular => Ok(components.to_vec()),
        EntryKind::Symlink => resolve(filesystem, device, components).map_err(|_| Error::NotAFile(components.to_vec())),
        _ => Err(Error::NotAFile(components.to_vec())),
    }
}

/// Write the file at `components` into the image at `path`, from the
/// pieces `receiver` yields, whole: every missing parent made, the
/// content streamed into a temporary beside the destination, any old
/// destination removed and the temporary renamed over it, and the
/// filesystem's metadata flushed. A parent that is not a directory
/// is [`Error::NotADirectory`]; a destination that is a directory is
/// [`Error::NotAFile`]; a piece that is the caller's error is
/// [`Error::Content`], the write abandoned. On any failure the
/// temporary is removed and the destination is as it was.
///
/// The whole write runs on the blocking pool, the receiver drained
/// by `blocking_recv`, so the writer is one thread and the content
/// is held one piece at a time.
pub async fn write_file(
    path: &Path,
    components: &[String],
    mut receiver: mpsc::Receiver<Result<Bytes, error::Error>>,
) -> Result<(), Error> {
    let image = path.to_path_buf();
    let components = components.to_vec();
    tokio::task::spawn_blocking(move || {
        let mut device = FileBackend::open(&image)?;
        let mut filesystem = Ext::open(&mut device)?;
        filesystem.replay_pending_journal(&mut device)?;
        let (name, parents) = components.split_last().expect("the handler refused an empty path");
        make_parents(&mut filesystem, &mut device, parents)?;
        let destination = inside(&components);
        if let Ok(attrs) = filesystem.getattr(&mut device, Path::new(&destination)) {
            if attrs.kind != EntryKind::Regular {
                return Err(Error::NotAFile(components.clone()));
            }
        }
        let mut temporary = parents.to_vec();
        temporary.push(format!(".{name}.{}", uuid::Uuid::new_v4()));
        let temporary = inside(&temporary);
        let written = fill(&mut filesystem, &mut device, &temporary, &mut receiver);
        if let Err(error) = written {
            let _ = filesystem.remove(&mut device, Path::new(&temporary));
            let _ = filesystem.flush(&mut device);
            return Err(error);
        }
        let placed = (|| {
            match filesystem.remove(&mut device, Path::new(&destination)) {
                Ok(()) | Err(fstool::Error::InvalidArgument(_)) => {}
                Err(error) => return Err(Error::Format(error)),
            }
            // The trait's rename, by name: `Ext` has an inherent one
            // of another shape.
            fstool::fs::Filesystem::rename(&mut filesystem, &mut device, Path::new(&temporary), Path::new(&destination))?;
            Ok::<(), Error>(())
        })();
        if let Err(error) = placed {
            let _ = filesystem.remove(&mut device, Path::new(&temporary));
            let _ = filesystem.flush(&mut device);
            return Err(error);
        }
        filesystem.flush(&mut device)?;
        Ok(())
    })
    .await
    .map_err(io::Error::other)?
}

/// Every directory of `parents` made, from the root down, where it is
/// not there already; one that is there and is not a directory is
/// [`Error::NotADirectory`].
fn make_parents(filesystem: &mut Ext, device: &mut dyn BlockDevice, parents: &[String]) -> Result<(), Error> {
    for depth in 1..=parents.len() {
        let at = &parents[..depth];
        match filesystem.getattr(device, Path::new(&inside(at))) {
            Ok(attrs) if attrs.kind == EntryKind::Dir => {}
            Ok(_) => return Err(Error::NotADirectory(at.to_vec())),
            Err(fstool::Error::InvalidArgument(_)) => {
                filesystem.create_dir(
                    device,
                    Path::new(&inside(at)),
                    FileMeta {
                        // rwxr-xr-x, as a directory a container makes is.
                        mode: 0o755,
                        ..FileMeta::default()
                    },
                )?;
            }
            Err(error) => return Err(Error::Format(error)),
        }
    }
    Ok(())
}

/// The temporary at `temporary` made and filled with every piece the
/// receiver yields, in order, and synced; a piece that is the
/// caller's error is [`Error::Content`].
fn fill(
    filesystem: &mut Ext,
    device: &mut dyn BlockDevice,
    temporary: &str,
    receiver: &mut mpsc::Receiver<Result<Bytes, error::Error>>,
) -> Result<(), Error> {
    let mut handle = filesystem.open_file_rw(
        device,
        Path::new(temporary),
        OpenFlags {
            create: true,
            truncate: true,
            append: false,
        },
        Some(FileMeta::default()),
    )?;
    while let Some(piece) = receiver.blocking_recv() {
        let bytes = piece.map_err(Error::Content)?;
        handle.write_all(&bytes)?;
    }
    handle.sync()?;
    Ok(())
}

/// The subtree at `components` of the image at `path`, as the nodes a
/// filetree snapshot carries, walked whole on the blocking pool.
/// Every directory's `changes` is `false`, since nothing watches an
/// image; a symlink's `path` is its target as components relative to
/// the root, made lexically as a watch makes one. Nothing at the
/// path is [`Error::Missing`]; anything but a directory there is
/// [`Error::NotADirectory`].
pub async fn tree(path: &Path, components: &[String]) -> Result<Vec<Node>, Error> {
    let image = path.to_path_buf();
    let components = components.to_vec();
    tokio::task::spawn_blocking(move || {
        let mut device = FileBackend::open(&image)?;
        let mut filesystem = Ext::open(&mut device)?;
        filesystem.replay_pending_journal(&mut device)?;
        match filesystem.getattr(&mut device, Path::new(&inside(&components))) {
            Ok(attrs) if attrs.kind == EntryKind::Dir => {}
            Ok(_) => return Err(Error::NotADirectory(components)),
            Err(fstool::Error::InvalidArgument(_)) => return Err(Error::Missing(components)),
            Err(error) => return Err(Error::Format(error)),
        }
        children(&mut filesystem, &mut device, &components)
    })
    .await
    .map_err(io::Error::other)?
}

/// The entries of the directory at `components`, each a node, a
/// directory's own children walked in turn.
fn children(filesystem: &mut Ext, device: &mut dyn BlockDevice, components: &[String]) -> Result<Vec<Node>, Error> {
    let entries = filesystem.list(device, Path::new(&inside(components)))?;
    let mut nodes = Vec::with_capacity(entries.len());
    for entry in entries {
        if entry.name == "." || entry.name == ".." {
            continue;
        }
        let mut below = components.to_vec();
        below.push(entry.name.clone());
        let attrs = filesystem.getattr(device, Path::new(&inside(&below)))?;
        let created_at = Some(u64::from(attrs.ctime));
        let modified_at = Some(u64::from(attrs.mtime));
        nodes.push(match entry.kind {
            EntryKind::Dir => Node::Directory {
                name: entry.name,
                created_at,
                modified_at,
                changes: false,
                children: children(filesystem, device, &below)?,
            },
            EntryKind::Symlink => {
                let target = filesystem.read_symlink(device, Path::new(&inside(&below)))?;
                Node::Symlink {
                    name: entry.name,
                    path: target_components(&below, &target.to_string_lossy()),
                    created_at,
                    modified_at,
                }
            }
            _ => Node::File {
                name: entry.name,
                size: Some(attrs.size),
                created_at,
                modified_at,
            },
        });
    }
    Ok(nodes)
}

/// A link's target as components relative to the root, lexically: an
/// absolute target from the root, a relative one from the link's
/// directory, `..` popping and `.` ignored — the shape a watch
/// reports, made from the image's own strings.
fn target_components(link: &[String], target: &str) -> Vec<String> {
    let mut components: Vec<String> = if target.starts_with('/') {
        Vec::new()
    } else {
        link[..link.len() - 1].to_vec()
    };
    for component in target.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                components.pop();
            }
            name => components.push(name.to_string()),
        }
    }
    components
}

/// A path inside the image, as the reader spells one: `/`-joined from
/// the root, whatever the host's separator, so it is built from
/// strings and never through [`PathBuf::join`].
fn inside(components: &[String]) -> String {
    format!("/{}", components.join("/"))
}

/// Where a stored volume's image is: `<store>/<identity>/<name>`.
pub fn image_path(store: &Path, client_identity: &str, name: &str) -> PathBuf {
    store.join(client_identity).join(name)
}
