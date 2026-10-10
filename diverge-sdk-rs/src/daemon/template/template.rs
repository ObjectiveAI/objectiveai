//! What an agent or a tool is made from, less its name, its
//! provider and its mounts.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::daemon::endpoints::agents::create::client::request::Image;

/// Everything an agent or a tool is made from that is the same for
/// every one made from it: what it is for, how to make one in words,
/// the image, the limits, the arguments. What is not here is what
/// differs one to the next — the name, the provider it runs on, and
/// every mount of providers' volumes — which the
/// [agent's](crate::daemon::endpoints::agents::create) or the
/// [tool's](crate::daemon::endpoints::tools::create) create states.
///
/// # One shape, typed
///
/// `Type` is the family's own single-value enum —
/// [`AgentType`](crate::daemon::endpoints::agents::templates::AgentType)
/// or
/// [`ToolType`](crate::daemon::endpoints::tools::templates::ToolType) —
/// so an agent template is `{"type":"agent",…}` and a tool template
/// `{"type":"tool",…}`, neither decoding as the other, and the two
/// hashing apart however alike the rest.
///
/// # No provider, so that it travels
///
/// A template is meant to be shared: handed from one caller to
/// another, published, made again anywhere and hashed the same. A
/// provider is one daemon's acquaintance, named by how that daemon
/// came to know it, and a template that named one would be that
/// daemon's alone. So the provider is the real agent's or tool's,
/// chosen at its create, and a template says nothing about where it
/// runs.
///
/// Its id is its hash: see [`template`](super). What a caller may
/// not choose is not here at all rather than here and ignored: the
/// container's name, its ports, its entrypoint and its environment
/// are the provider's. The account a container runs under is not
/// here either: it is the create's, so that one template makes
/// containers of different standing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Template<Type> {
    /// What the template is for: the one value the family's type
    /// admits, `agent` or `tool`. Inside the hash, so the two kinds
    /// never hash the same.
    pub r#type: Type,
    /// How to make a container from this, in words, for whoever does:
    /// what the create has to supply that a template cannot name — the
    /// volumes and FUSE mounts it needs and at what paths, the account
    /// it has to run under and what that account has to be able to do,
    /// which agents it has to be able to message and which have to
    /// reach it, and anything else about deploying it. The template is
    /// shareable and this travels with it, inside the hash; a template
    /// with nothing to say leaves it absent. Not handed to the
    /// container, and read by nothing but a person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The image: a digest and its references. See [`Image`]; the id
    /// hashes it with the references absent, see [`hashed`](Self::hashed).
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
    /// What the image is told once, as the image defines it, for the
    /// container's life.
    ///
    /// A JSON value, because this crate does not know what an image
    /// takes — a model, tools, a tool server's own knobs — and a wire
    /// that typed it would have to be revised for every image that ever
    /// ran. It is handed to the container and not read here.
    pub arguments: Value,
}

impl<Type: Clone> Template<Type> {
    /// The template as its id hashes it: the same, with the image's
    /// references absent, since where the bytes may be fetched is not
    /// what the template is.
    pub fn hashed(&self) -> Template<Type> {
        Template {
            image: self.image.hashed(),
            ..self.clone()
        }
    }
}
