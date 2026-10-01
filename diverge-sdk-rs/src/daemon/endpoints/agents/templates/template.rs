//! What an agent is made from, less its name and its mounts.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::daemon::endpoints::agents::create::client::request::Image;
use crate::daemon::endpoints::agents::logs::server::response::Identity;

/// Everything an agent is made from that is the same for every agent
/// made from it: the image, the limits, the provider pin, the
/// arguments. What is not here is what differs agent to agent — the
/// name, and the mounts — which the agent's
/// [`create`](crate::daemon::endpoints::agents::create) states.
///
/// Its id is its hash: see [`templates`](super). What a caller may
/// not choose is not here at all rather than here and ignored: the
/// container's name, its ports, its entrypoint and its environment
/// are the provider's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Template {
    /// The image: a name and a digest. See [`Image`].
    pub image: Image,
    /// How much memory the container may have, in BYTES.
    ///
    /// A ceiling, not a hint. A process that exceeds what the
    /// container is allowed is killed by the kernel rather than told
    /// — no failed allocation to catch, no warning first — and the
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
    /// not counted, so a container starts at nothing however large
    /// the image is. It does not govern the mounts: a volume is
    /// storage that already existed, with a size of its own.
    ///
    /// Bytes rather than megabytes, for the reason
    /// [`memory`](Self::memory) gives.
    pub disk: u64,
    /// The one provider every agent made from this runs on, by its
    /// [`Identity`] as the daemon knows it. Absent, an agent runs on
    /// whichever provider the daemon chooses, and can mount no
    /// volume: a volume is a provider's own and does not carry
    /// across, so an agent with state on a provider's disk is an
    /// agent of that provider. Which volumes of that provider an
    /// agent mounts is the agent's create's to say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<Identity>,
    /// What the image is told once, as the image defines it, for the
    /// agent's life.
    ///
    /// A JSON value, because this crate does not know what an image
    /// takes — a model, tools, an image's own knobs — and a wire that
    /// typed it would have to be revised for every image that ever
    /// ran. It is handed to the container and not read here.
    pub arguments: Value,
}
