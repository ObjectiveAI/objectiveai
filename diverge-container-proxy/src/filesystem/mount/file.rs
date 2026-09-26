//! A file mount: a single regular file that can be read and written
//! in place, but never deleted, moved or replaced by a rename, its
//! bytes the caller's.
//!
//! One inode, the root, a regular file: `getattr` asks the caller
//! what it holds; `open` asks nothing and hands back a
//! [`handle`](super::handles) that remembers only that it is open;
//! every `read` and `write` is its own ask with the kernel's offset
//! and size, carried as it comes; a truncation and a change of mode,
//! owner or times are asks of their own; `flush`, `fsync` and
//! `release` ask nothing, there being nothing held back. Each ask
//! carries an empty path — the mount is the file. A caller that will
//! not take a change answers the refusal, and the program sees the
//! call fail. Nothing else ever arrives, because the root is a file:
//! a rename or unlink of the mount point, or a rename onto it, is the
//! kernel's to refuse in the directory around it.

use std::ffi::OsStr;
use std::time::SystemTime;

use diverge_sdk::shared::containers::fuse;
use fuser::{
    BsdFileFlags, Errno, FileAttr, FileHandle, Filesystem, INodeNo, KernelConfig, LockOwner,
    OpenAccMode, OpenFlags, ReplyAttr, ReplyData, ReplyEmpty, ReplyEntry, ReplyOpen, ReplyWrite,
    TimeOrNow, WriteFlags,
};

use super::asks::Asks;
use super::attrs::{self, ATTR_TTL, OPEN};
use super::handles::Handles;

/// The one file.
pub struct MountedFile {
    asks: Asks,
    /// When the mount was made: the times of a file the caller holds
    /// nothing for yet.
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

    /// The file's attributes, as the caller says now: nothing held
    /// yet is an empty file, and a directory is the caller's mistake,
    /// not this mount's.
    fn attr(&self) -> Result<FileAttr, Errno> {
        match self.asks.stat("")? {
            Some(stat) if stat.kind == fuse::Kind::File => Ok(attrs::attr(INodeNo::ROOT, &stat)),
            Some(_) => Err(Errno::EIO),
            None => Ok(attrs::absent(INodeNo::ROOT, fuse::Kind::File, super::FILE_MODE, self.born)),
        }
    }
}

impl Filesystem for MountedFile {
    fn init(&mut self, _req: &fuser::Request, config: &mut KernelConfig) -> std::io::Result<()> {
        attrs::init(config);
        Ok(())
    }

    fn lookup(&self, _req: &fuser::Request, _parent: INodeNo, _name: &OsStr, reply: ReplyEntry) {
        // The root is a file: there is nothing under it.
        reply.error(Errno::ENOTDIR);
    }

    fn getattr(&self, _req: &fuser::Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        if ino != INodeNo::ROOT {
            reply.error(Errno::ENOENT);
            return;
        }
        match self.attr() {
            Ok(attr) => reply.attr(&ATTR_TTL, &attr),
            Err(errno) => reply.error(errno),
        }
    }

    fn setattr(
        &self,
        _req: &fuser::Request,
        ino: INodeNo,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        atime: Option<TimeOrNow>,
        mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<FileHandle>,
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
        let result = (|| {
            if let Some(size) = size {
                self.asks.truncate("", size)?;
            }
            let attrs = attrs::attrs(mode, uid, gid, atime, mtime);
            if !attrs.is_empty() {
                self.asks.setattr("", attrs)?;
            }
            self.attr()
        })();
        match result {
            Ok(attr) => reply.attr(&ATTR_TTL, &attr),
            Err(errno) => reply.error(errno),
        }
    }

    fn open(&self, _req: &fuser::Request, ino: INodeNo, flags: OpenFlags, reply: ReplyOpen) {
        if ino != INodeNo::ROOT {
            reply.error(Errno::ENOENT);
            return;
        }
        let writable = matches!(flags.acc_mode(), OpenAccMode::O_WRONLY | OpenAccMode::O_RDWR);
        if flags.0 & libc::O_TRUNC != 0 {
            if let Err(errno) = self.asks.truncate("", 0) {
                reply.error(errno);
                return;
            }
        }
        let fh = self.handles.open(String::new(), writable);
        reply.opened(fh, OPEN);
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
        let result = self.handles.get(fh).and_then(|(path, _)| self.asks.read(&path, offset, size));
        match result {
            // Nothing held yet reads as an empty file.
            Ok(Some(bytes)) => reply.data(&bytes),
            Ok(None) => reply.data(&[]),
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
        let result = self.handles.get(fh).and_then(|(path, writable)| {
            if !writable {
                return Err(Errno::EBADF);
            }
            self.asks.write(&path, offset, data)
        });
        match result {
            Ok(()) => reply.written(data.len() as u32),
            Err(errno) => reply.error(errno),
        }
    }

    fn flush(&self, _req: &fuser::Request, _ino: INodeNo, _fh: FileHandle, _lock_owner: LockOwner, reply: ReplyEmpty) {
        // Every write landed as it came.
        reply.ok();
    }

    fn fsync(&self, _req: &fuser::Request, _ino: INodeNo, _fh: FileHandle, _datasync: bool, reply: ReplyEmpty) {
        reply.ok();
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
        self.handles.release(fh);
        reply.ok();
    }
}
