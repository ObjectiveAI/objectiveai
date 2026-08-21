//! What a client's request frame carries for a laboratory run.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::container::request::{Image, Mount};

/// Ask a provider to create a container.
///
/// Everything here is what a caller may CHOOSE. What it may not
/// choose is not here at all rather than here and ignored: the
/// container's name, its published ports, its labels and its
/// entrypoint are the provider's, because they are how the provider
/// reaches the container afterwards.
///
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frame {
    /// The image, and who supplies it.
    ///
    /// See [`Image`]. Which variant it is decides how the image is
    /// named, which is why the source and the name are one field
    /// rather than two that only make sense together.
    pub image: Image,
    /// How much memory the container may have, in BYTES.
    ///
    /// A ceiling, not a hint. A process that exceeds what the
    /// container is allowed is killed by the kernel rather than told —
    /// no failed allocation to catch, no warning first — and the
    /// container will not see this number in its own
    /// `/proc/meminfo`, which reports the host's. An image that sizes
    /// itself off what it thinks it has will size itself wrong.
    ///
    /// Bytes rather than megabytes because a unit that has to be
    /// spelled out in prose is a unit half of everyone gets wrong.
    pub memory: u64,
    /// How much the container may WRITE, in BYTES.
    ///
    /// Its own filesystem only — what it adds to or changes over the
    /// image it came from. The image's layers are read-only and are
    /// not counted, so a container starts at nothing however large the
    /// image is.
    ///
    /// # It does not govern [`mounts`](Self::mounts)
    ///
    /// A mount is storage that already existed, with a size of its own
    /// that a [`volume`](crate::endpoints::volumes) stated when it was
    /// made. A number here that silently applied to it would be this
    /// request deciding how much of somebody else's volume a container
    /// may fill.
    ///
    /// Bytes rather than megabytes, for the reason
    /// [`memory`](Self::memory) gives.
    pub disk: u64,
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
    /// What to call this container.
    ///
    /// A label the caller chooses, for whatever inside the container
    /// wants to know what it is. The laboratory MCP is the case that
    /// prompted it — it names its own server after this and surfaces
    /// it verbatim in its instructions, so an agent can say which
    /// container it is talking about.
    ///
    /// # It is not the handle
    ///
    /// [`Id`](crate::endpoints::laboratories::run::server::response::Frame::Id)
    /// is, and the provider mints it. That separation is deliberate:
    /// a
    /// [`connect`](crate::endpoints::laboratories::connect) names a
    /// container by id, and so does a
    /// [`transfer`](crate::shared::container::transfer)'s destination,
    /// so an identifier a caller could CHOOSE would be an identifier
    /// another caller could guess. Guessing one is how you reach a
    /// container nobody gave you.
    ///
    /// Which is why nothing requires this to be unique. Two containers
    /// may share a name and will not share an id, and no provider has
    /// to reconcile that — it never resolves anything by this.
    ///
    /// May be empty, which is a container that does not care what it
    /// is called.
    pub name: String,
    /// Where an agent that connects into this container starts, as
    /// path components from the container's root.
    ///
    /// Empty means the root, which is also what omitting it means.
    ///
    /// # A starting point, not a boundary
    ///
    /// An agent that arrives here can leave. `cd ..` goes one above
    /// this, and keeps going. Nothing about this field confines
    /// anything, and reading it as a jail is the one way to get it
    /// badly wrong.
    ///
    /// What confines an agent is [`mounts`](Self::mounts) — the
    /// container sees the image's own filesystem and whatever
    /// directories were mounted into it, and nothing else exists to
    /// walk to. This only decides where the walking begins, so a
    /// caller that mounted something at `/workspace` can have agents
    /// land there instead of at `/`.
    ///
    /// # Not the container's working directory
    ///
    /// The image's `WORKDIR` is its author's business and this does
    /// not touch it. The entrypoint runs where the image says; this is
    /// about the agents that connect in afterwards, which is a
    /// different thing happening at a different time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub initial_cwd: Vec<String>,
}

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them
/// — [`ClientFrame::Request`](crate::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::endpoints) for the whole
/// allocation. The values are chosen across modules that do not know
/// about each other, so the table is the only place they can be seen
/// at once.
const TAG: u8 = 2;

/// JSON, matching [`images::check`](crate::endpoints::images::check) rather than
/// the postcard [`volumes`](crate::endpoints::volumes) uses.
///
/// One of these is sent per container rather than per filesystem
/// event, so there is no throughput to optimize for — and it names an
/// image the same way [`images::check`](crate::endpoints::images::check) does,
/// which is reason enough for the two to look alike on the wire.
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

/// A laboratory run request that could not be read.
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
                f.write_str("laboratory run request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(
                    f,
                    "expected laboratory run request tag {TAG}, \
                     found {tag}"
                )
            }
            FrameError::Body(error) => {
                write!(f, "laboratory run request did not parse: {error}")
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
