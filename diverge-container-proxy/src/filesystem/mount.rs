//! A FUSE mount: a single regular file that can be read and — unless
//! read-only — overwritten, but never moved or deleted, its bytes the
//! caller's.
//!
//! The SDK's [`filesystem`](diverge_provider_sdk::container_proxy::filesystem)
//! module states the semantics; this is the filesystem that keeps
//! them. One inode, the root, a regular file: `getattr` asks the
//! caller for the bytes and answers their length, `open` reads them
//! into a buffer of the handle's own, `read` and `write` work the
//! buffer, and `flush`, `fsync` and `release` of a changed buffer
//! store it whole — each ask one [`fuse`](diverge_provider_sdk::container_proxy::fuse)
//! exchange on `/requests`, carrying the mount's id. A read-only
//! mount refuses every write twice over: the kernel's `ro` option
//! turns writes away before they reach here, and every write path
//! here answers `EROFS` regardless, so the rule holds even where a
//! kernel lets an open through. Everything else is the trait's
//! default — and nothing else ever arrives, because the root is a
//! file: a rename or unlink of the mount point is the kernel's to
//! refuse, in the directory around it.
//!
//! FUSE calls the filesystem on the session's own thread, and the
//! caller is asked on the runtime: every ask is the runtime handle's
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

/// Make the file's parents, the file itself if absent, and mount over
/// it: read-only at the kernel too when the mount says so.
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
        .mode(unix::mode(mount.readonly))
        .open(&path)?;

    let mut config = fuser::Config::default();
    config.mount_options = vec![
        fuser::MountOption::FSName("diverge-fuse".to_string()),
        fuser::MountOption::DefaultPermissions,
        fuser::MountOption::NoAtime,
        if mount.readonly {
            fuser::MountOption::RO
        } else {
            fuser::MountOption::RW
        },
    ];
    config.acl = fuser::SessionACL::All;
    config.n_threads = Some(1);
    let session = fuser::Session::new(
        unix::MountedFile::new(requests, handle, &mount.id, mount.readonly),
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
        format!("FUSE mounts need FUSE, which this host has not: {}", mount.id),
    ))
}

#[cfg(unix)]
mod unix {
    use std::collections::BTreeMap;
    use std::ffi::OsStr;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, SystemTime};

    use diverge_provider_sdk::container_proxy::fuse;
    use diverge_provider_sdk::container_proxy::requests::request::Request;
    use fuser::{
        BsdFileFlags, Errno, FileAttr, FileHandle, FileType, Filesystem, FopenFlags, INodeNo,
        LockOwner, OpenAccMode, OpenFlags, ReplyAttr, ReplyData, ReplyEmpty, ReplyEntry,
        ReplyOpen, ReplyWrite, TimeOrNow, WriteFlags,
    };
    use tokio::runtime::Handle;

    use crate::ask;
    use crate::requests::Requests;

    /// How long the kernel may believe an attribute: not at all — the
    /// caller is the truth, and the file is small.
    const ATTR_TTL: Duration = Duration::ZERO;

    /// The file's mode: owner read, and write unless read-only.
    pub fn mode(readonly: bool) -> u32 {
        if readonly { 0o400 } else { 0o600 }
    }

    /// The one file.
    pub struct MountedFile {
        requests: Arc<Requests>,
        handle: Handle,
        /// The mount's id, echoed on every ask.
        id: String,
        /// Whether every write is refused.
        readonly: bool,
        /// Every time the file has: the proxy's start.
        born: SystemTime,
        /// The open handles, by their number.
        open: Mutex<BTreeMap<u64, Open>>,
        /// The next handle number.
        next: AtomicU64,
    }

    /// One open handle: its own copy of the file.
    struct Open {
        buffer: Vec<u8>,
        writable: bool,
        dirty: bool,
    }

    impl MountedFile {
        pub fn new(requests: Arc<Requests>, handle: Handle, id: &str, readonly: bool) -> Self {
            MountedFile {
                requests,
                handle,
                id: id.to_string(),
                readonly,
                born: SystemTime::now(),
                open: Mutex::new(BTreeMap::new()),
                next: AtomicU64::new(1),
            }
        }

        /// The file's bytes, from the caller: empty when it holds
        /// nothing under the id yet.
        fn get(&self) -> Result<Vec<u8>, Errno> {
            let request = Request::FuseRead(fuse::read::request::Request { id: &self.id });
            let answer = self
                .handle
                .block_on(ask::ask(&self.requests, request))
                .map_err(|_| Errno::EIO)?;
            match fuse::read::response::Frame::decode(&answer) {
                Ok(fuse::read::response::Frame::Present(bytes)) => Ok(bytes.to_vec()),
                Ok(fuse::read::response::Frame::Missing) => Ok(Vec::new()),
                Ok(fuse::read::response::Frame::Error(_)) | Err(_) => Err(Errno::EIO),
            }
        }

        /// The file's bytes, stored with the caller, whole.
        fn set(&self, bytes: &[u8]) -> Result<(), Errno> {
            if self.readonly {
                return Err(Errno::EROFS);
            }
            let request = Request::FuseWrite(fuse::write::request::Request {
                id: &self.id,
                bytes,
            });
            let answer = self
                .handle
                .block_on(ask::ask(&self.requests, request))
                .map_err(|_| Errno::EIO)?;
            match fuse::write::response::Frame::decode(&answer) {
                Ok(fuse::write::response::Frame::Ok) => Ok(()),
                Ok(fuse::write::response::Frame::Error(_)) | Err(_) => Err(Errno::EIO),
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
                perm: mode(self.readonly) as u16,
                nlink: 1,
                uid: 0,
                gid: 0,
                rdev: 0,
                blksize: 4096,
                flags: 0,
            }
        }

        /// Store a changed handle's buffer as the file, and mark it
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

        /// The size a `getattr` would answer: the handle's buffer, or
        /// the caller's file.
        fn getattr_size(&self, fh: Option<FileHandle>) -> Result<u64, Errno> {
            match fh {
                Some(fh) => {
                    let open = self.open.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    open.get(&fh.0)
                        .map(|handle| handle.buffer.len() as u64)
                        .ok_or(Errno::EBADF)
                }
                None => self.get().map(|bytes| bytes.len() as u64),
            }
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
            match self.getattr_size(fh) {
                Ok(size) => reply.attr(&ATTR_TTL, &self.attr(size)),
                Err(errno) => reply.error(errno),
            }
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
            if self.readonly {
                reply.error(Errno::EROFS);
                return;
            }
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
                // `truncate(2)` with no handle: the file itself,
                // resized and stored at once; every open write handle
                // follows, so a later flush does not resurrect the old
                // length.
                None => {
                    let mut bytes = match self.get() {
                        Ok(bytes) => bytes,
                        Err(errno) => {
                            reply.error(errno);
                            return;
                        }
                    };
                    bytes.resize(length, 0);
                    if let Err(errno) = self.set(&bytes) {
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
            if self.readonly && (writable || truncate) {
                reply.error(Errno::EROFS);
                return;
            }
            let buffer = if truncate {
                Vec::new()
            } else {
                match self.get() {
                    Ok(bytes) => bytes,
                    Err(errno) => {
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
            if self.readonly {
                reply.error(Errno::EROFS);
                return;
            }
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
            // A changed handle is stored on its way out — the kernel's
            // `flush` normally came first and left it clean. A
            // release's error reaches nobody, so the reply is always
            // ok.
            let _ = self.flush_handle(fh);
            self.open
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&fh.0);
            reply.ok();
        }
    }
}
