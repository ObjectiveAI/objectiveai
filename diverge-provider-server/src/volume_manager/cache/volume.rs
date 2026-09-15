//! One volume, and what is known about it.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::Mutex;

use super::{Walked, walk};

/// Where a volume lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// In the store at this index of the configuration's list, made
    /// by a create.
    Stored { store: usize },
    /// A fixed volume of the configuration, existing before any
    /// client asked.
    Fixed,
}

/// One volume: what a listing knows without looking, and what a walk
/// finds once asked.
///
/// The size and the creation time are read with the volume and held;
/// the size changes only by an edit. What a walk finds — the bytes in
/// use and the hash of the content — is not known until
/// [`check`](Self::check) walks, and is forgotten when the volume is
/// [`mounted`](Self::mounted), because a container writes and nothing
/// else does.
///
/// # And the SDK's lock
///
/// The lock the SDK takes before a mount, a stat, an edit or a
/// delete, and gives back after, lives here beside everything else
/// known about the volume, as an atomic flag: [`lock`](Self::lock)
/// sets it if it was clear, [`unlock`](Self::unlock) clears it,
/// [`locked`](Self::locked) reads it, and none of them waits on
/// anything. The manager's
/// [`Handle`](crate::volume_manager::Handle) is what the SDK calls
/// them through.
#[derive(Debug)]
pub struct Volume {
    name: String,
    root: PathBuf,
    place: Place,
    /// The SDK's lock: `true` while a run, a stat, an edit or a
    /// delete has the volume.
    lock: AtomicBool,
    /// How big the volume may be, in bytes: what its create asked for,
    /// as its last edit left it. `None` for a fixed volume, whose size
    /// no store recorded.
    bytes: Mutex<Option<u64>>,
    /// When the volume came into being, in seconds since the Unix
    /// epoch.
    created: u64,
    /// What the last walk found, if one has happened since the volume
    /// was read or last mounted.
    walked: Mutex<Option<Walked>>,
}

impl Volume {
    pub fn new(name: &str, root: PathBuf, place: Place, bytes: Option<u64>, created: u64) -> Self {
        Volume {
            name: name.to_string(),
            root,
            place,
            lock: AtomicBool::new(false),
            bytes: Mutex::new(bytes),
            created,
            walked: Mutex::new(None),
        }
    }

    /// The name a listing gives it.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The directory that is the volume.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where it lives.
    pub fn place(&self) -> Place {
        self.place
    }

    /// When it came into being, in seconds since the Unix epoch.
    pub fn created(&self) -> u64 {
        self.created
    }

    /// How big it may be, in bytes; `None` for a fixed volume.
    pub async fn bytes(&self) -> Option<u64> {
        *self.bytes.lock().await
    }

    /// An edit: the volume may now be this big.
    pub async fn resize(&self, bytes: u64) {
        *self.bytes.lock().await = Some(bytes);
    }

    /// What a walk finds: the bytes in use and the hash of the content.
    ///
    /// Walked once and held: a second check answers from the first,
    /// and a check that arrives while a walk runs waits for that walk
    /// rather than starting another. Held until the volume is
    /// [`mounted`](Self::mounted). A walk that fails holds nothing,
    /// and the next check walks again.
    pub async fn check(&self) -> io::Result<Walked> {
        let mut walked = self.walked.lock().await;
        if let Some(walked) = walked.as_ref() {
            return Ok(walked.clone());
        }
        let found = walk::walk(&self.root).await?;
        *walked = Some(found.clone());
        Ok(found)
    }

    /// The volume is being mounted into a container: whatever a walk
    /// found is forgotten, since the container may write from now on
    /// and nothing reports when. The next [`check`](Self::check) walks
    /// again. Separate from the SDK's lock, which a stat takes too:
    /// a stat must not wipe the walk it just made.
    pub async fn mounted(&self) {
        *self.walked.lock().await = None;
    }

    /// Take the SDK's lock: `true` is the flag set, and it was clear;
    /// `false` is the flag already set, and nothing changed. One
    /// compare-and-swap, so two takers at once cannot both succeed.
    pub fn lock(&self) -> bool {
        self.lock
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Give the SDK's lock back: the flag cleared. `true` is a flag
    /// that was set; `false` is one that was already clear.
    pub fn unlock(&self) -> bool {
        self.lock.swap(false, Ordering::AcqRel)
    }

    /// Whether the SDK's lock is held, as of now.
    pub fn locked(&self) -> bool {
        self.lock.load(Ordering::Acquire)
    }
}
