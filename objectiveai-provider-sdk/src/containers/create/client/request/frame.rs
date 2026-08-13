//! What a client's request frame carries for a container creation.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::Mount;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Ask a provider to create a container.
///
/// Everything here is what a caller may CHOOSE. What it may not
/// choose is not here at all rather than here and ignored: the
/// container's name, its published ports, its labels and its
/// entrypoint are the provider's, because they are how the provider
/// reaches the container afterwards.
///
/// # Creating is not starting
///
/// This gets a container that exists. Running it is a separate ask,
/// which is what makes the resources here checkable before anything
/// executes — a provider that cannot honour
/// [`memory`](Self::memory) says so now rather than by killing
/// something later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Frame {
    /// The image's repository path — `library/nginx`, `myorg/myimage`.
    ///
    /// Kept alongside the digest because a digest alone is not
    /// resolvable: every registry API is repository-scoped, and there
    /// is no lookup from a digest to wherever it lives.
    pub name: String,
    /// The image's manifest digest, `<algorithm>:<hex>`.
    ///
    /// What actually identifies the image. Unlike a tag, it cannot be
    /// repointed at different content — so a container created twice
    /// from the same digest is created twice from the same bytes.
    ///
    /// No registry, for the reason
    /// [`images::check`](crate::images::check) gives: where a provider
    /// gets the image is the provider's business, and a caller naming
    /// a registry it cannot reach would be asserting something it has
    /// no standing to assert.
    pub digest: String,
    /// How much memory the container may have, in BYTES.
    ///
    /// A ceiling, not a hint. A process past it is killed by the
    /// kernel — no failed allocation to catch, no warning first — and
    /// the container will not see this number in its own
    /// `/proc/meminfo`, which reports the host's. An image that sizes
    /// itself off what it thinks it has will size itself wrong.
    ///
    /// Bytes rather than megabytes because a unit that has to be
    /// spelled out in prose is a unit half of everyone gets wrong.
    pub memory: u64,
    /// The environment, name to value.
    ///
    /// A map rather than a list of `KEY=VALUE` strings, so one name
    /// cannot appear twice with values that contradict each other.
    /// Ordered, so the same environment always serializes identically.
    ///
    /// A provider may reserve names and will win any collision — it
    /// has to, since some of what a container needs to talk back is
    /// delivered this way. Which names are reserved is a provider's to
    /// state.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub environment: IndexMap<String, String>,
    /// Directories from a listing to make visible inside the
    /// container.
    ///
    /// Ordered, and a provider applies them in order. Nothing here
    /// prevents two mounts from overlapping; what that means is the
    /// provider's to decide, and it is the reason the order is
    /// preserved rather than sorted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mounts: Vec<Mount>,
    /// The working directory processes start in, as path components
    /// from the container's root.
    ///
    /// Empty means the root, which is also what it means to omit this.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub working_directory: Vec<String>,
}

/// This frame's tag among the scope-opening requests.
///
/// `0` is the agentic loop, `1` the image check, `2` the filesystem
/// listing, `3` the watch. The values are allocated across five
/// modules that do not know about each other, so a sixth request has
/// to look at all of them.
const TAG: u8 = 4;

/// JSON, matching [`images::check`](crate::images::check) rather than
/// the postcard [`filesystem`](crate::filesystem) uses.
///
/// It carries the same thing images check does — a repository name and
/// a digest — and one of these is sent per container rather than per
/// filesystem event, so there is no volume to optimize for and no
/// reason for two sides of one concern to be encoded differently.
impl Encode for Frame {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != TAG {
            return Err(FrameError::UnexpectedTag(*tag));
        }
        serde_json::from_slice(rest).map_err(FrameError::Body)
    }
}

/// A container creation request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag naming some other request.
    ///
    /// A reader that dispatched on the tag will not see this. One that
    /// assumed which request it held, and was wrong, will — which is
    /// the point of checking a tag rather than skipping it.
    UnexpectedTag(u8),
    /// The request did not parse.
    Body(serde_json::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("container creation request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(
                    f,
                    "expected container creation request tag {TAG}, \
                     found {tag}"
                )
            }
            FrameError::Body(error) => {
                write!(f, "container creation request did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Body(error) => Some(error),
            FrameError::Empty | FrameError::UnexpectedTag(_) => None,
        }
    }
}
