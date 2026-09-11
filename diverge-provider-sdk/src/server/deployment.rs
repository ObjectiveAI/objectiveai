//! What every container asks for, whatever is going to run in it.

use indexmap::IndexMap;

use super::mount::Mount;
use crate::shared::containers::request::IdentityMount;

/// A container to put somewhere, minus the image.
///
/// What the [`containers`](crate::endpoints::containers) family means
/// by "a container" once the differences between its kinds are set
/// aside. An agent container runs an agent, a tool container serves
/// tools — and none of that is here, because none of it changes how
/// the container is deployed.
///
/// # It is built, not passed through
///
/// No endpoint's request frame becomes one of these. A handler reads
/// its own request and fills this in — the limits and the mounts as
/// the caller sent them, the environment as the handler completes it.
///
/// Which is the point. A deployer that took a request frame would take
/// one per scope, and be that many deployers.
///
/// # The image is not here
///
/// It is an argument to whichever
/// [`ContainerDeployer`](super::container_deployer::ContainerDeployer)
/// method is called, because the method IS the source. Carrying an
/// [`Image`](crate::shared::containers::request::Image) here as well
/// would let a caller hand
/// [`Client`](crate::shared::containers::request::Image::Client) to the
/// method that pulls from a registry, which is a contradiction nothing
/// would catch.
///
/// # What is deliberately absent
///
/// A plugin's `arguments` and `identity` are not container facts. The
/// endpoint already says a provider delivers them through the
/// environment "by its own reserved names", so a handler folds them in
/// and what arrives here is [`environment`](Self::environment).
///
/// The FUSE mounts are not here: the proxy makes those itself, one
/// request each from the handler once the container is up, and all a
/// deployer owes them is what every container gets — `/dev/fuse` and
/// the privilege to mount in its own namespace.
///
/// Its ports are not here either, because there is exactly one and it
/// is always the same: the proxy's
/// [`OUTSIDE_PORT`](crate::container_proxy::OUTSIDE_PORT), which a
/// deployer makes reachable on every container it deploys and reports
/// as the container's [`address`](super::container::Container::address).
/// The entrypoint's own port is the proxy's to reach, on the loopback
/// inside, and nothing outside ever dials it.
///
/// The [`client_identity`](Mount::client_identity) on a
/// [`Mount`] is not that `identity` and does not contradict this. That
/// one is what a plugin is told about its caller; this one is what the
/// provider knows about its own, and it is here because a volume name
/// cannot be resolved without it.
///
/// A laboratory's `initial_cwd` is where an agent's shell starts, not
/// where the container's entrypoint runs — `mcp_plugin` says as much
/// about its own absence of one: "the image's own `WORKDIR` governs
/// its entrypoint".
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Deployment {
    /// How much memory the container may have, in BYTES.
    ///
    /// A ceiling, not a hint. What happens at it is the provider's —
    /// the kernel kills the process, and nothing in this protocol
    /// promises the container is told first.
    pub memory: u64,
    /// How much the container may WRITE, in BYTES.
    ///
    /// Its own filesystem only: what it adds to or changes over the
    /// image it came from. The image's layers are read-only and are
    /// not counted, so a container starts at nothing however large the
    /// image is.
    ///
    /// Not the [`mounts`](Self::mounts). A volume has its own size,
    /// which a caller set when it made the volume, and writing into
    /// one spends that rather than this.
    pub disk: u64,
    /// The environment, name to value.
    ///
    /// Ordered, so the same environment always deploys identically,
    /// and a map rather than `KEY=VALUE` strings so one name cannot
    /// appear twice with values that contradict each other.
    ///
    /// Everything a caller asked for, plus whatever the handler added
    /// on its own account. A provider that reserves names for its own
    /// use has already applied them by the time this arrives.
    pub environment: IndexMap<String, String>,
    /// The volumes to make visible inside it.
    ///
    /// Empty for a container that takes none.
    ///
    /// Not the [`VolumeMount`](crate::shared::containers::request::VolumeMount) a
    /// caller sent. [`server::Mount`](Mount) is that one plus the
    /// caller it came from, because a volume's name is unique within
    /// the caller it was listed to and means nothing on its own — so a
    /// handler pairs each with whoever it authenticated before putting
    /// it here.
    pub mounts: Vec<Mount>,
    /// The files the caller mounts by identity, each read-only at its
    /// path, from the provider's
    /// [`ContentStore`](super::content_store::ContentStore).
    ///
    /// Every identity here is held by the time a deploy is asked for:
    /// the handler fetched what the store lacked first, so a deployer
    /// binds and never fetches. The wire type, unchanged — a path and
    /// an identity are all a bind needs.
    pub identity_file_mounts: Vec<IdentityMount>,
    /// The directories the caller mounts by identity, likewise.
    pub identity_directory_mounts: Vec<IdentityMount>,
}
