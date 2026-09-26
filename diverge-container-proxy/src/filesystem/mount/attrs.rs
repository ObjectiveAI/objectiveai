//! What the kernel is told about an entry, from what the caller said;
//! and what the caller is told to set, from what the kernel asked.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use diverge_provider_sdk::CHUNK_SIZE;
use diverge_provider_sdk::shared::containers::fuse;
use diverge_provider_sdk::shared::containers::fuse::stat::Stat;
use fuser::{FileAttr, FileType, FopenFlags, INodeNo, KernelConfig, TimeOrNow};

/// How long the kernel may believe an attribute: not at all — the
/// caller is the truth.
pub const ATTR_TTL: Duration = Duration::ZERO;

/// How an open file is opened: direct, so the kernel keeps no page
/// cache over it and every `read(2)` and `write(2)` reaches the mount
/// with its own offset and size; parallel, so two writers on one file
/// need not wait on each other here — they wait on the caller.
pub const OPEN: FopenFlags = FopenFlags::FOPEN_DIRECT_IO.union(FopenFlags::FOPEN_PARALLEL_DIRECT_WRITES);

/// The kernel's connection, set: a read or a write of at most one
/// chunk, so no ask outgrows a message, and no readahead past that.
pub fn init(config: &mut KernelConfig) {
    let _ = config.set_max_write(CHUNK_SIZE as u32);
    let _ = config.set_max_readahead(CHUNK_SIZE as u32);
}

/// A caller's stat as the kernel's attributes of `ino`.
pub fn attr(ino: INodeNo, stat: &Stat) -> FileAttr {
    let (kind, nlink) = match stat.kind {
        fuse::Kind::File => (FileType::RegularFile, 1),
        fuse::Kind::Directory => (FileType::Directory, 2),
    };
    FileAttr {
        ino,
        size: stat.size,
        blocks: stat.size.div_ceil(512),
        atime: at(stat.atime),
        mtime: at(stat.mtime),
        ctime: at(stat.ctime),
        crtime: at(stat.ctime),
        kind,
        perm: (stat.mode & 0o7777) as u16,
        nlink,
        uid: stat.uid,
        gid: stat.gid,
        rdev: 0,
        blksize: 4096,
        flags: 0,
    }
}

/// The attributes of an entry the caller holds nothing for yet: a
/// file of no bytes, or the directory the mount point is, with the
/// mount point's own mode and the moment it was mounted, owned by
/// root.
pub fn absent(ino: INodeNo, kind: fuse::Kind, mode: u32, born: SystemTime) -> FileAttr {
    let now = time(born);
    attr(
        ino,
        &Stat {
            kind,
            size: 0,
            mode,
            uid: 0,
            gid: 0,
            atime: now,
            mtime: now,
            ctime: now,
        },
    )
}

/// A wire time as the kernel's.
fn at(time: fuse::Time) -> SystemTime {
    UNIX_EPOCH + Duration::new(time.secs, time.nanos)
}

/// A kernel time as the wire's; before 1970 is 1970.
pub fn time(time: SystemTime) -> fuse::Time {
    let since = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    fuse::Time {
        secs: since.as_secs(),
        nanos: since.subsec_nanos(),
    }
}

/// What a `setattr` asked, as the caller's attributes: the mode's
/// permission bits, the owner, the group, and the two times — `now`
/// being this moment.
pub fn attrs(mode: Option<u32>, uid: Option<u32>, gid: Option<u32>, atime: Option<TimeOrNow>, mtime: Option<TimeOrNow>) -> fuse::Attrs {
    let now = |time: TimeOrNow| match time {
        TimeOrNow::SpecificTime(time) => self::time(time),
        TimeOrNow::Now => self::time(SystemTime::now()),
    };
    fuse::Attrs {
        mode: mode.map(|mode| mode & 0o7777),
        uid,
        gid,
        atime: atime.map(now),
        mtime: mtime.map(now),
    }
}
