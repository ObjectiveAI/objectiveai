//! A file mount: a FUSE mount of a single regular file that can be
//! read and overwritten but never moved or deleted, its bytes kept
//! under a vault key.
//!
//! The SDK's [`filesystem`](diverge_provider_sdk::container_proxy::filesystem)
//! module states the semantics; this is the filesystem that keeps
//! them. One inode, the root, a regular file: `getattr` asks the
//! vault for the size, `open` reads the value into a buffer of the
//! handle's own (locking the key first for a writer), `read` and
//! `write` work the buffer, and `flush`, `fsync` and `release` of a
//! changed buffer set it whole. Everything else is the trait's
//! default — and nothing else ever arrives, because the root is a
//! file: a rename or unlink of the mount point is the kernel's to
//! refuse, in the directory around it.
//!
//! FUSE calls the filesystem on the session's own thread, and the
//! vault is asked on the runtime: every ask is the runtime handle's
//! `block_on`, which is what a handle is for from a thread that is
//! not the runtime's. Unix only — the check host is not the
//! container — and [`mount`] on anything else refuses.

use std::io;
use std::sync::Arc;

use diverge_provider_sdk::container_proxy::filesystem::Mount;
use tokio::runtime::Handle;

use crate::requests::Requests;

/// A mount, held: dropping it unmounts.
pub struct Mounted {
    #[cfg(unix)]
    _session: fuser::BackgroundSession,
}

/// Make the file's parents, the file itself if absent (mode `0600`),
/// and mount over it.
#[cfg(unix)]
pub fn mount(requests: Arc<Requests>, handle: Handle, mount: &Mount) -> io::Result<Mounted> {
    use std::os::unix::fs::OpenOptionsExt as _;
    use std::path::PathBuf;

    let mut path = PathBuf::from("/");
    path.extend(&mount.path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .mode(0o600)
        .open(&path)?;

    let mut config = fuser::Config::default();
    config.mount_options = vec![
        fuser::MountOption::FSName("diverge-vault".to_string()),
        fuser::MountOption::DefaultPermissions,
        fuser::MountOption::NoAtime,
    ];
    config.acl = fuser::SessionACL::All;
    config.n_threads = Some(1);
    let session = fuser::Session::new(
        unix::MountedFile::new(requests, handle, &mount.key),
        &path,
        &config,
    )?;
    Ok(Mounted {
        _session: session.spawn()?,
    })
}

/// Not the container: there is no FUSE here, and a mount is refused.
#[cfg(not(unix))]
pub fn mount(_requests: Arc<Requests>, _handle: Handle, mount: &Mount) -> io::Result<Mounted> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        format!("file mounts need FUSE, which this host has not: {}", mount.key),
    ))
}

#[cfg(unix)]
mod unix {
    use std::collections::BTreeMap;
    use std::ffi::OsStr;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, SystemTime};

    use diverge_provider_sdk::container_proxy::requests::request::Request;
    use diverge_provider_sdk::container_proxy::vault;
    use fuser::{
        BsdFileFlags, Errno, FileAttr, FileHandle, FileType, Filesystem, FopenFlags, INodeNo,
        LockOwner, OpenAccMode, OpenFlags, ReplyAttr, ReplyData, ReplyEmpty, ReplyEntry,
        ReplyOpen, ReplyWrite, TimeOrNow, WriteFlags,
    };
    use tokio::runtime::Handle;

    use crate::requests::Requests;
    use crate::vault::ask;

    /// How long a writer's lock is granted for, in seconds.
    const LOCK_TTL: u32 = 300;

    /// How long the kernel may believe an attribute: not at all — the
    /// vault is the truth, and the file is small.
    const ATTR_TTL: Duration = Duration::ZERO;

    /// The one file.
    pub struct MountedFile {
        requests: Arc<Requests>,
        handle: Handle,
        key: String,
        /// Every time the file has: the proxy's start.
        born: SystemTime,
        /// The open handles, by their number.
        open: Mutex<BTreeMap<u64, Open>>,
        /// The next handle number.
        next: AtomicU64,
    }

    /// One open handle: its own copy of the value.
    struct Open {
        buffer: Vec<u8>,
        writable: bool,
        dirty: bool,
    }

    impl MountedFile {
        pub fn new(requests: Arc<Requests>, handle: Handle, key: &str) -> Self {
            MountedFile {
                requests,
                handle,
                key: key.to_string(),
                born: SystemTime::now(),
                open: Mutex::new(BTreeMap::new()),
                next: AtomicU64::new(1),
            }
        }

        /// The key's value: empty when the vault holds nothing.
        fn get(&self) -> Result<Vec<u8>, Errno> {
            let request = Request::VaultGet(vault::get::request::Request { key: &self.key });
            let answer = self
                .handle
                .block_on(ask::ask(&self.requests, request))
                .map_err(|_| Errno::EIO)?;
            match vault::get::response::Frame::decode(&answer) {
                Ok(vault::get::response::Frame::Present(bytes)) => Ok(bytes.to_vec()),
                Ok(vault::get::response::Frame::Missing) => Ok(Vec::new()),
                Ok(vault::get::response::Frame::Error(_)) | Err(_) => Err(Errno::EIO),
            }
        }

        /// The key's value, replaced whole.
        fn set(&self, value: &[u8]) -> Result<(), Errno> {
            let request = Request::VaultSet(vault::set::request::Request {
                key: &self.key,
                value,
            });
            self.answered(request, Errno::EIO)
        }

        /// The key, locked for a writer.
        fn lock(&self) -> Result<(), Errno> {
            let request = Request::VaultLock(vault::lock::request::Request {
                key: &self.key,
                ttl: LOCK_TTL,
            });
            self.answered(request, Errno::EAGAIN)
        }

        /// The key, unlocked. A failure has nobody to tell.
        fn unlock(&self) {
            let request = Request::VaultUnlock(vault::unlock::request::Request { key: &self.key });
            let _ = self.answered(request, Errno::EIO);
        }

        /// An ask answered ok or error; the error as `refused`.
        fn answered(&self, request: Request<'_>, refused: Errno) -> Result<(), Errno> {
            let answer = self
                .handle
                .block_on(ask::ask(&self.requests, request))
                .map_err(|_| Errno::EIO)?;
            match vault::response::Frame::decode(&answer) {
                Ok(vault::response::Frame::Ok) => Ok(()),
                Ok(vault::response::Frame::Error(_)) => Err(refused),
                Err(_) => Err(Errno::EIO),
            }
        }

        /// The file's attributes at a size.
        fn attr(&self, size: u64) -> FileAttr {
            FileAttr {
                ino: INodeNo::ROOT,
                size,
                blocks: size.div_ceil(512),
                atime: self.born,
                mtime: self.born,
                ctime: self.born,
                crtime: self.born,
                kind: FileType::RegularFile,
                perm: 0o600,
                nlink: 1,
                uid: 0,
                gid: 0,
                rdev: 0,
                blksize: 4096,
                flags: 0,
            }
        }

        /// Set a changed handle's buffer as the value, and mark it
        /// clean. An unchanged handle is nothing to do.
        fn flush_handle(&self, fh: FileHandle) -> Result<(), Errno> {
            let buffer = {
                let open = self.open.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                match open.get(&fh.0) {
                    Some(handle) if handle.dirty => handle.buffer.clone(),
                    Some(_) => return Ok(()),
                    None => return Err(Errno::EBADF),
                }
            };
            self.set(&buffer)?;
            let mut open = self.open.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(handle) = open.get_mut(&fh.0) {
                handle.dirty = false;
            }
            Ok(())
        }
    }

    impl Filesystem for MountedFile {
        fn lookup(&self, _req: &fuser::Request, _parent: INodeNo, _name: &OsStr, reply: ReplyEntry) {
            // The root is a file: there is nothing under it.
            reply.error(Errno::ENOTDIR);
        }

        fn getattr(&self, _req: &fuser::Request, ino: INodeNo, fh: Option<FileHandle>, reply: ReplyAttr) {
            if ino != INodeNo::ROOT {
                reply.error(Errno::ENOENT);
                return;
            }
            let size = match fh {
                Some(fh) => {
                    let open = self.open.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    match open.get(&fh.0) {
                        Some(handle) => handle.buffer.len() as u64,
                        None => {
                            reply.error(Errno::EBADF);
                            return;
                        }
                    }
                }
                None => match self.get() {
                    Ok(value) => value.len() as u64,
                    Err(errno) => {
                        reply.error(errno);
                        return;
                    }
                },
            };
            reply.attr(&ATTR_TTL, &self.attr(size));
        }

        fn setattr(
            &self,
            _req: &fuser::Request,
            ino: INodeNo,
            _mode: Option<u32>,
            _uid: Option<u32>,
            _gid: Option<u32>,
            size: Option<u64>,
            _atime: Option<TimeOrNow>,
            _mtime: Option<TimeOrNow>,
            _ctime: Option<SystemTime>,
            fh: Option<FileHandle>,
            _crtime: Option<SystemTime>,
            _chgtime: Option<SystemTime>,
            _bkuptime: Option<SystemTime>,
            _flags: Option<BsdFileFlags>,
            reply: ReplyAttr,
        ) {
            if ino != INodeNo::ROOT {
                reply.error(Errno::ENOENT);
                return;
            }
            let Some(size) = size else {
                // Times, mode, owner: accepted, and the file stays as
                // it is.
                match self.getattr_size(fh) {
                    Ok(size) => reply.attr(&ATTR_TTL, &self.attr(size)),
                    Err(errno) => reply.error(errno),
                }
                return;
            };
            let length = size as usize;
            match fh {
                // A handle's own truncation: its buffer, and it is
                // dirty until flushed.
                Some(fh) => {
                    let mut open = self.open.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    match open.get_mut(&fh.0) {
                        Some(handle) => {
                            handle.buffer.resize(length, 0);
                            handle.dirty = true;
                        }
                        None => {
                            reply.error(Errno::EBADF);
                            return;
                        }
                    }
                }
                // `truncate(2)` with no handle: the value itself,
                // resized and set at once; every open write handle
                // follows, so a later flush does not resurrect the
                // old length.
                None => {
                    let mut value = match self.get() {
                        Ok(value) => value,
                        Err(errno) => {
                            reply.error(errno);
                            return;
                        }
                    };
                    value.resize(length, 0);
                    if let Err(errno) = self.set(&value) {
                        reply.error(errno);
                        return;
                    }
                    let mut open = self.open.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    for handle in open.values_mut().filter(|handle| handle.writable) {
                        handle.buffer.resize(length, 0);
                    }
                }
            }
            reply.attr(&ATTR_TTL, &self.attr(size));
        }

        fn open(&self, _req: &fuser::Request, ino: INodeNo, flags: OpenFlags, reply: ReplyOpen) {
            if ino != INodeNo::ROOT {
                reply.error(Errno::ENOENT);
                return;
            }
            let writable = matches!(
                flags.acc_mode(),
                OpenAccMode::O_WRONLY | OpenAccMode::O_RDWR
            );
            let truncate = flags.0 & libc::O_TRUNC != 0;
            if writable {
                if let Err(errno) = self.lock() {
                    reply.error(errno);
                    return;
                }
            }
            let buffer = if truncate {
                Vec::new()
            } else {
                match self.get() {
                    Ok(value) => value,
                    Err(errno) => {
                        if writable {
                            self.unlock();
                        }
                        reply.error(errno);
                        return;
                    }
                }
            };
            let fh = self.next.fetch_add(1, Ordering::Relaxed);
            self.open
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .insert(
                    fh,
                    Open {
                        buffer,
                        writable,
                        dirty: truncate,
                    },
                );
            reply.opened(FileHandle(fh), FopenFlags::empty());
        }

        fn read(
            &self,
            _req: &fuser::Request,
            _ino: INodeNo,
            fh: FileHandle,
            offset: u64,
            size: u32,
            _flags: OpenFlags,
            _lock_owner: Option<LockOwner>,
            reply: ReplyData,
        ) {
            let open = self.open.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(handle) = open.get(&fh.0) else {
                reply.error(Errno::EBADF);
                return;
            };
            let start = (offset as usize).min(handle.buffer.len());
            let end = start.saturating_add(size as usize).min(handle.buffer.len());
            reply.data(&handle.buffer[start..end]);
        }

        fn write(
            &self,
            _req: &fuser::Request,
            _ino: INodeNo,
            fh: FileHandle,
            offset: u64,
            data: &[u8],
            _write_flags: WriteFlags,
            _flags: OpenFlags,
            _lock_owner: Option<LockOwner>,
            reply: ReplyWrite,
        ) {
            let mut open = self.open.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(handle) = open.get_mut(&fh.0) else {
                reply.error(Errno::EBADF);
                return;
            };
            if !handle.writable {
                reply.error(Errno::EBADF);
                return;
            }
            let start = offset as usize;
            let end = start + data.len();
            if handle.buffer.len() < end {
                handle.buffer.resize(end, 0);
            }
            handle.buffer[start..end].copy_from_slice(data);
            handle.dirty = true;
            reply.written(data.len() as u32);
        }

        fn flush(
            &self,
            _req: &fuser::Request,
            _ino: INodeNo,
            fh: FileHandle,
            _lock_owner: LockOwner,
            reply: ReplyEmpty,
        ) {
            match self.flush_handle(fh) {
                Ok(()) => reply.ok(),
                Err(errno) => reply.error(errno),
            }
        }

        fn fsync(
            &self,
            _req: &fuser::Request,
            _ino: INodeNo,
            fh: FileHandle,
            _datasync: bool,
            reply: ReplyEmpty,
        ) {
            match self.flush_handle(fh) {
                Ok(()) => reply.ok(),
                Err(errno) => reply.error(errno),
            }
        }

        fn release(
            &self,
            _req: &fuser::Request,
            _ino: INodeNo,
            fh: FileHandle,
            _flags: OpenFlags,
            _lock_owner: Option<LockOwner>,
            _flush: bool,
            reply: ReplyEmpty,
        ) {
            // A changed handle is set on its way out — the kernel's
            // `flush` normally came first and left it clean — and a
            // writer's lock is released whatever the set did. A
            // release's error reaches nobody, so the reply is always
            // ok.
            let _ = self.flush_handle(fh);
            let removed = self
                .open
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&fh.0);
            if let Some(handle) = removed {
                if handle.writable {
                    self.unlock();
                }
            }
            reply.ok();
        }
    }

    impl MountedFile {
        /// The size a `getattr` would answer: the handle's buffer, or
        /// the vault's value.
        fn getattr_size(&self, fh: Option<FileHandle>) -> Result<u64, Errno> {
            match fh {
                Some(fh) => {
                    let open = self.open.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    open.get(&fh.0)
                        .map(|handle| handle.buffer.len() as u64)
                        .ok_or(Errno::EBADF)
                }
                None => self.get().map(|value| value.len() as u64),
            }
        }
    }
}
