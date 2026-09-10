//! A directory mount: a real filesystem rooted at the path, the whole
//! tree the caller's.
//!
//! The root is a directory that cannot be deleted or moved — it is
//! the mount point. Every entry under it is the caller's: `lookup`
//! and `getattr` are a stat of the entry, `readdir` a listing of the
//! directory; a
//! file's `open` reads its bytes into a [`handle`](super::handles) of
//! its own and its changed close stores them whole, exactly as on a
//! file mount; `create` makes an empty file with the caller at once —
//! so `O_EXCL`, and a `stat` before the first close, see it — and
//! opens it; `mkdir`, `unlink`, `rmdir` and `rename` are the caller's
//! own asks. A rename onto an existing file replaces it whole, which
//! is how a program that saves by temporary-and-rename overwrites.
//! `RENAME_NOREPLACE` is honoured by a look first; `RENAME_EXCHANGE`
//! is `ENOTSUP`. Symbolic links, hard links and device nodes are
//! `EPERM`. Times, mode and owner are accepted and change nothing.
//! Attributes are never cached, so every `stat` is the caller's
//! current answer — nine bytes, never the file.
//!
//! Inodes are numbers this filesystem hands out for paths as the
//! kernel looks them up, kept while the kernel holds a lookup count
//! and freed when it forgets; a rename re-keys every number under
//! the moved entry, so a handle or an inode taken before the move
//! still names the entry after it.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, SystemTime};

use fuser::{
    BsdFileFlags, Errno, FileAttr, FileHandle, FileType, Filesystem, FopenFlags, Generation,
    INodeNo, LockOwner, OpenAccMode, OpenFlags, RenameFlags, ReplyAttr, ReplyCreate,
    ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry, ReplyOpen, ReplyWrite, TimeOrNow,
    WriteFlags,
};

use super::asks::{Asks, Stat};
use super::handles::Handles;

/// How long the kernel may believe an attribute: not at all — the
/// caller is the truth.
const ATTR_TTL: Duration = Duration::ZERO;

/// Every inode's generation: this filesystem never reuses a number
/// for a different entry within one mount, so one generation serves.
const GENERATION: Generation = Generation(0);

/// The tree.
pub struct MountedDirectory {
    asks: Asks,
    /// Every time an entry has: the proxy's start.
    born: SystemTime,
    handles: Handles,
    inodes: Mutex<Inodes>,
}

/// The inode table: numbers for paths, and the kernel's lookup counts.
struct Inodes {
    nodes: BTreeMap<u64, Node>,
    numbers: BTreeMap<String, u64>,
    next: u64,
}

/// One numbered entry.
struct Node {
    /// The entry's path inside the mount; empty for the root.
    path: String,
    /// How many lookups the kernel has not yet forgotten.
    lookups: u64,
}

impl Inodes {
    fn new() -> Self {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            INodeNo::ROOT.0,
            Node {
                path: String::new(),
                lookups: 1,
            },
        );
        let mut numbers = BTreeMap::new();
        numbers.insert(String::new(), INodeNo::ROOT.0);
        Inodes {
            nodes,
            numbers,
            next: INodeNo::ROOT.0 + 1,
        }
    }

    fn path(&self, ino: u64) -> Option<String> {
        self.nodes.get(&ino).map(|node| node.path.clone())
    }

    /// The path's number, allocated if it has none; `lookup` adds one
    /// to the count the kernel will forget.
    fn number(&mut self, path: &str, lookup: bool) -> u64 {
        let ino = match self.numbers.get(path) {
            Some(ino) => *ino,
            None => {
                let ino = self.next;
                self.next += 1;
                self.numbers.insert(path.to_string(), ino);
                self.nodes.insert(
                    ino,
                    Node {
                        path: path.to_string(),
                        lookups: 0,
                    },
                );
                ino
            }
        };
        if lookup {
            if let Some(node) = self.nodes.get_mut(&ino) {
                node.lookups += 1;
            }
        }
        ino
    }

    /// The kernel forgot `count` lookups; at none the number is freed.
    /// The root is never freed.
    fn forget(&mut self, ino: u64, count: u64) {
        if ino == INodeNo::ROOT.0 {
            return;
        }
        let freed = match self.nodes.get_mut(&ino) {
            Some(node) => {
                node.lookups = node.lookups.saturating_sub(count);
                node.lookups == 0
            }
            None => false,
        };
        if freed {
            if let Some(node) = self.nodes.remove(&ino) {
                self.numbers.remove(&node.path);
            }
        }
    }

    /// An entry moved: every number on it, or under it, follows.
    fn rename(&mut self, from: &str, to: &str) {
        let mut numbers = BTreeMap::new();
        for node in self.nodes.values_mut() {
            if node.path == from {
                node.path = to.to_string();
            } else if let Some(rest) = node.path.strip_prefix(from).and_then(|rest| rest.strip_prefix('/')) {
                node.path = format!("{to}/{rest}");
            }
        }
        for (ino, node) in &self.nodes {
            numbers.insert(node.path.clone(), *ino);
        }
        self.numbers = numbers;
    }
}

/// The parent's path and the entry's name, from an entry's path.
fn split(path: &str) -> (&str, &str) {
    match path.rsplit_once('/') {
        Some((parent, name)) => (parent, name),
        None => ("", path),
    }
}

/// The path of `name` under `parent`: one component, as the kernel
/// hands it, that is not `.`, `..` or empty and holds no separator.
fn child(parent: &str, name: &OsStr) -> Result<String, Errno> {
    let name = name.to_str().ok_or(Errno::EINVAL)?;
    if name.is_empty() || name == "." || name == ".." || name.contains('/') {
        return Err(Errno::EINVAL);
    }
    Ok(if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    })
}

impl MountedDirectory {
    pub fn new(asks: Asks) -> Self {
        MountedDirectory {
            asks,
            born: SystemTime::now(),
            handles: Handles::new(),
            inodes: Mutex::new(Inodes::new()),
        }
    }

    fn inodes(&self) -> MutexGuard<'_, Inodes> {
        self.inodes.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The path an inode names.
    fn path(&self, ino: INodeNo) -> Result<String, Errno> {
        self.inodes().path(ino.0).ok_or(Errno::ENOENT)
    }

    /// The path's number, allocated if it has none.
    fn number(&self, path: &str, lookup: bool) -> u64 {
        self.inodes().number(path, lookup)
    }

    /// What is at the path, as the caller says now: the root is a
    /// directory; anything else is the caller's one stat.
    fn stat(&self, path: &str) -> Result<Stat, Errno> {
        if path.is_empty() {
            return Ok(Stat::Directory);
        }
        self.asks.stat(path)?.ok_or(Errno::ENOENT)
    }

    /// An entry's attributes.
    fn attr(&self, ino: u64, stat: &Stat) -> FileAttr {
        let readonly = self.asks.readonly();
        let (kind, size, perm, nlink) = match stat {
            Stat::File(size) => (FileType::RegularFile, *size, super::file_mode(readonly), 1),
            Stat::Directory => (FileType::Directory, 0, super::directory_mode(readonly), 2),
        };
        FileAttr {
            ino: INodeNo(ino),
            size,
            blocks: size.div_ceil(512),
            atime: self.born,
            mtime: self.born,
            ctime: self.born,
            crtime: self.born,
            kind,
            perm: perm as u16,
            nlink,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 4096,
            flags: 0,
        }
    }

    /// The attributes a `getattr` answers: a handle's buffer length,
    /// or the caller's current answer.
    fn getattr(&self, ino: INodeNo, fh: Option<FileHandle>) -> Result<FileAttr, Errno> {
        if let Some(fh) = fh {
            let size = self.handles.size(fh)?;
            return Ok(self.attr(ino.0, &Stat::File(size)));
        }
        let path = self.path(ino)?;
        let stat = self.stat(&path)?;
        Ok(self.attr(ino.0, &stat))
    }
}

impl Filesystem for MountedDirectory {
    fn lookup(&self, _req: &fuser::Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let result = self.path(parent).and_then(|parent| {
            let path = child(&parent, name)?;
            let stat = self.stat(&path)?;
            let ino = self.number(&path, true);
            Ok(self.attr(ino, &stat))
        });
        match result {
            Ok(attr) => reply.entry(&ATTR_TTL, &attr, GENERATION),
            Err(errno) => reply.error(errno),
        }
    }

    fn forget(&self, _req: &fuser::Request, ino: INodeNo, nlookup: u64) {
        self.inodes().forget(ino.0, nlookup);
    }

    fn getattr(&self, _req: &fuser::Request, ino: INodeNo, fh: Option<FileHandle>, reply: ReplyAttr) {
        match self.getattr(ino, fh) {
            Ok(attr) => reply.attr(&ATTR_TTL, &attr),
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
        let Some(size) = size else {
            // Times, mode, owner: accepted, and the entry stays as it
            // is.
            match self.getattr(ino, fh) {
                Ok(attr) => reply.attr(&ATTR_TTL, &attr),
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
            // A handle's own truncation: its buffer, dirty until
            // flushed.
            Some(fh) => self.handles.resize(fh, length),
            // `truncate(2)` with no handle: the file itself, resized
            // and stored at once; every open write handle follows.
            None => self.path(ino).and_then(|path| {
                let mut bytes = self.asks.read(&path)?.ok_or(Errno::ENOENT)?;
                bytes.resize(length, 0);
                self.asks.write(&path, &bytes)?;
                self.handles.resize_writable(&path, length);
                Ok(())
            }),
        };
        match result {
            Ok(()) => reply.attr(&ATTR_TTL, &self.attr(ino.0, &Stat::File(size))),
            Err(errno) => reply.error(errno),
        }
    }

    fn open(&self, _req: &fuser::Request, ino: INodeNo, flags: OpenFlags, reply: ReplyOpen) {
        let writable = matches!(
            flags.acc_mode(),
            OpenAccMode::O_WRONLY | OpenAccMode::O_RDWR
        );
        let truncate = flags.0 & libc::O_TRUNC != 0;
        if self.asks.readonly() && (writable || truncate) {
            reply.error(Errno::EROFS);
            return;
        }
        let result = self.path(ino).and_then(|path| {
            let buffer = if truncate {
                // A truncating open of a directory is the kernel's to
                // refuse; a plain one reaches here as ENOENT below.
                Vec::new()
            } else {
                self.asks.read(&path)?.ok_or(Errno::ENOENT)?
            };
            Ok(self.handles.open(path, buffer, writable, truncate))
        });
        match result {
            Ok(fh) => reply.opened(fh, FopenFlags::empty()),
            Err(errno) => reply.error(errno),
        }
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
        // A changed handle is stored on its way out; a release's error
        // reaches nobody, so the reply is always ok.
        let _ = self.handles.flush(fh, |path, bytes| self.asks.write(path, bytes));
        self.handles.release(fh);
        reply.ok();
    }

    fn readdir(&self, _req: &fuser::Request, ino: INodeNo, _fh: FileHandle, offset: u64, mut reply: ReplyDirectory) {
        let listed = match self.path(ino).and_then(|path| {
            let listed = self.asks.list(&path)?.ok_or(Errno::ENOENT)?;
            let parent = if path.is_empty() {
                INodeNo::ROOT.0
            } else {
                self.number(split(&path).0, false)
            };
            let mut entries: Vec<(u64, FileType, String)> = Vec::with_capacity(listed.len() + 2);
            entries.push((ino.0, FileType::Directory, ".".to_string()));
            entries.push((parent, FileType::Directory, "..".to_string()));
            for entry in listed {
                let kind = if entry.directory {
                    FileType::Directory
                } else {
                    FileType::RegularFile
                };
                let number = self.number(&child(&path, OsStr::new(&entry.name))?, false);
                entries.push((number, kind, entry.name));
            }
            Ok(entries)
        }) {
            Ok(entries) => entries,
            Err(errno) => {
                reply.error(errno);
                return;
            }
        };
        // An entry's offset is its position plus one: the kernel asks
        // again from the last offset it saw, and zero is the start.
        for (index, (number, kind, name)) in listed.into_iter().enumerate().skip(offset as usize) {
            if reply.add(INodeNo(number), index as u64 + 1, kind, name) {
                break;
            }
        }
        reply.ok();
    }

    fn fsyncdir(&self, _req: &fuser::Request, _ino: INodeNo, _fh: FileHandle, _datasync: bool, reply: ReplyEmpty) {
        // Every change was stored with the caller as it landed.
        reply.ok();
    }

    fn create(
        &self,
        _req: &fuser::Request,
        parent: INodeNo,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        flags: i32,
        reply: ReplyCreate,
    ) {
        let writable = matches!(flags & libc::O_ACCMODE, libc::O_WRONLY | libc::O_RDWR);
        let result = self.path(parent).and_then(|parent| {
            let path = child(&parent, name)?;
            if flags & libc::O_EXCL != 0 && self.stat(&path).is_ok() {
                return Err(Errno::EEXIST);
            }
            // Made with the caller at once, empty, so the entry exists
            // before its first close: an `O_EXCL` by another opener,
            // or a `stat`, sees it.
            self.asks.write(&path, &[])?;
            let ino = self.number(&path, true);
            let fh = self.handles.open(path, Vec::new(), writable, false);
            Ok((self.attr(ino, &Stat::File(0)), fh))
        });
        match result {
            Ok((attr, fh)) => reply.created(&ATTR_TTL, &attr, GENERATION, fh, FopenFlags::empty()),
            Err(errno) => reply.error(errno),
        }
    }

    fn mkdir(&self, _req: &fuser::Request, parent: INodeNo, name: &OsStr, _mode: u32, _umask: u32, reply: ReplyEntry) {
        let result = self.path(parent).and_then(|parent| {
            let path = child(&parent, name)?;
            self.asks.mkdir(&path)?;
            let ino = self.number(&path, true);
            Ok(self.attr(ino, &Stat::Directory))
        });
        match result {
            Ok(attr) => reply.entry(&ATTR_TTL, &attr, GENERATION),
            Err(errno) => reply.error(errno),
        }
    }

    fn unlink(&self, _req: &fuser::Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        match self.path(parent).and_then(|parent| self.asks.remove(&child(&parent, name)?)) {
            Ok(()) => reply.ok(),
            Err(errno) => reply.error(errno),
        }
    }

    fn rmdir(&self, _req: &fuser::Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        match self.path(parent).and_then(|parent| self.asks.remove(&child(&parent, name)?)) {
            Ok(()) => reply.ok(),
            Err(errno) => reply.error(errno),
        }
    }

    fn rename(
        &self,
        _req: &fuser::Request,
        parent: INodeNo,
        name: &OsStr,
        newparent: INodeNo,
        newname: &OsStr,
        flags: RenameFlags,
        reply: ReplyEmpty,
    ) {
        if self.asks.readonly() {
            reply.error(Errno::EROFS);
            return;
        }
        if flags.contains(RenameFlags::RENAME_EXCHANGE) {
            reply.error(Errno::ENOTSUP);
            return;
        }
        let result = (|| {
            let from = child(&self.path(parent)?, name)?;
            let to = child(&self.path(newparent)?, newname)?;
            if flags.contains(RenameFlags::RENAME_NOREPLACE) && self.stat(&to).is_ok() {
                return Err(Errno::EEXIST);
            }
            self.asks.rename(&from, &to)?;
            self.inodes().rename(&from, &to);
            self.handles.retarget(&from, &to);
            Ok(())
        })();
        match result {
            Ok(()) => reply.ok(),
            Err(errno) => reply.error(errno),
        }
    }

    fn mknod(
        &self,
        _req: &fuser::Request,
        _parent: INodeNo,
        _name: &OsStr,
        _mode: u32,
        _umask: u32,
        _rdev: u32,
        reply: ReplyEntry,
    ) {
        // Nothing but regular files and directories: the caller serves
        // no device, and a fifo would be a file that is not a message.
        reply.error(Errno::EPERM);
    }
}
