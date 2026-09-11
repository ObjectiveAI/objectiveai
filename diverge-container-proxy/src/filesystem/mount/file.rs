//! A file mount: a single regular file that can be read and — unless
//! read-only — overwritten in place, but never deleted, moved or
//! replaced by a rename, its bytes the caller's.
//!
//! One inode, the root, a regular file: `getattr` asks the caller
//! what it holds and answers the length, `open` reads the bytes into a
//! [`handle`](super::handles) of its own, `read` and `write` work the
//! buffer, and `flush`, `fsync` and `release` of a changed buffer
//! store it whole — each ask carrying the mount's id and an empty
//! path. A read-only mount refuses every write twice over: the
//! kernel's `ro` option turns writes away before they reach here, and
//! every write path here answers `EROFS` regardless. Nothing else
//! ever arrives, because the root is a file: a rename or unlink of
//! the mount point, or a rename onto it, is the kernel's to refuse in
//! the directory around it.

use std::ffi::OsStr;
use std::time::{Duration, SystemTime};

use fuser::{
    BsdFileFlags, Errno, FileAttr, FileHandle, FileType, Filesystem, FopenFlags, INodeNo,
    LockOwner, OpenAccMode, OpenFlags, ReplyAttr, ReplyData, ReplyEmpty, ReplyEntry,
    ReplyOpen, ReplyWrite, TimeOrNow, WriteFlags,
};

use super::asks::{Asks, Stat};
use super::handles::Handles;

/// How long the kernel may believe an attribute: not at all — the
/// caller is the truth, and the file is small.
const ATTR_TTL: Duration = Duration::ZERO;

/// The one file.
pub struct MountedFile {
    asks: Asks,
    /// Every time the file has: the proxy's start.
    born: SystemTime,
    handles: Handles,
}

impl MountedFile {
    pub fn new(asks: Asks) -> Self {
        MountedFile {
            asks,
            born: SystemTime::now(),
            handles: Handles::new(),
        }
    }

    /// The file's bytes, from the caller: empty when it holds nothing
    /// under the id yet.
    fn bytes(&self) -> Result<Vec<u8>, Errno> {
        Ok(self.asks.read("")?.unwrap_or_default())
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
            perm: super::file_mode(self.asks.readonly()) as u16,
            nlink: 1,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 4096,
            flags: 0,
        }
    }

    /// The size a `getattr` would answer: the handle's buffer, or the
    /// caller's stat — nothing held yet is an empty file, and a
    /// directory is the caller's mistake, not this mount's.
    fn size(&self, fh: Option<FileHandle>) -> Result<u64, Errno> {
        match fh {
            Some(fh) => self.handles.size(fh),
            None => match self.asks.stat("")? {
                Some(Stat::File(size)) => Ok(size),
                Some(Stat::Directory) => Err(Errno::EIO),
                None => Ok(0),
            },
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
        match self.size(fh) {
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
            // Times, mode, owner: accepted, and the file stays as it
            // is.
            match self.size(fh) {
                Ok(size) => reply.attr(&ATTR_TTL, &self.attr(size)),
                Err(errno) => reply.error(errno),
            }
            return;
        };
        if self.asks.readonly() {
            reply.error(Errno::EROFS);
            return;
        }
        let length = size as usize;
        let result = match fh {
            // A handle's own truncation: its buffer, and it is dirty
            // until flushed.
            Some(fh) => self.handles.resize(fh, length),
            // `truncate(2)` with no handle: the file itself, resized
            // and stored at once; every open write handle follows.
            None => self.bytes().and_then(|mut bytes| {
                bytes.resize(length, 0);
                self.asks.write("", &bytes)?;
                self.handles.resize_writable("", length);
                Ok(())
            }),
        };
        match result {
            Ok(()) => reply.attr(&ATTR_TTL, &self.attr(size)),
            Err(errno) => reply.error(errno),
        }
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
        if self.asks.readonly() && (writable || truncate) {
            reply.error(Errno::EROFS);
            return;
        }
        let buffer = if truncate {
            Vec::new()
        } else {
            match self.bytes() {
                Ok(bytes) => bytes,
                Err(errno) => {
                    reply.error(errno);
                    return;
                }
            }
        };
        let fh = self.handles.open(String::new(), buffer, writable, truncate);
        reply.opened(fh, FopenFlags::empty());
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
        match self.handles.read(fh, offset, size) {
            Ok(bytes) => reply.data(&bytes),
            Err(errno) => reply.error(errno),
        }
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
        if self.asks.readonly() {
            reply.error(Errno::EROFS);
            return;
        }
        match self.handles.write(fh, offset, data) {
            Ok(written) => reply.written(written),
            Err(errno) => reply.error(errno),
        }
    }

    fn flush(&self, _req: &fuser::Request, _ino: INodeNo, fh: FileHandle, _lock_owner: LockOwner, reply: ReplyEmpty) {
        match self.handles.flush(fh, |path, bytes| self.asks.write(path, bytes)) {
            Ok(()) => reply.ok(),
            Err(errno) => reply.error(errno),
        }
    }

    fn fsync(&self, _req: &fuser::Request, _ino: INodeNo, fh: FileHandle, _datasync: bool, reply: ReplyEmpty) {
        match self.handles.flush(fh, |path, bytes| self.asks.write(path, bytes)) {
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
        // `flush` normally came first and left it clean. A release's
        // error reaches nobody, so the reply is always ok.
        let _ = self.handles.flush(fh, |path, bytes| self.asks.write(path, bytes));
        self.handles.release(fh);
        reply.ok();
    }
}
