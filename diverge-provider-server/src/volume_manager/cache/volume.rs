//! One volume, and what is known about it.

use std::io;
use std::path::{Path, PathBuf};

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
#[derive(Debug)]
pub struct Volume {
    name: String,
    root: PathBuf,
    place: Place,
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
    /// again.
    pub async fn mounted(&self) {
        *self.walked.lock().await = None;
    }
}
