//! What every container asks for, whatever is going to run in it.

use indexmap::IndexMap;

use super::mount::Mount;

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
/// the caller sent them, the ports as the provider arranges them.
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
/// Its ports ARE, and are [`ports`](Self::ports).
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
    /// The ports inside the container that must be reachable.
    ///
    /// Container ports, not host ones. HOW a provider makes one
    /// reachable is its own business — publishing it to a loopback port
    /// it picks, routing to an address on a bridge, something a cloud
    /// runtime does that resembles neither — and nothing here says.
    ///
    /// # Why they are declared rather than asked for later
    ///
    /// Because on the runtimes that need publishing, publishing happens
    /// when the container is CREATED and cannot be added afterwards. So
    /// a port not named here may be unreachable for the container's
    /// whole life, and nothing would be able to fix it.
    ///
    /// Rootless Podman is the case that settles it. A rootless
    /// container has no address the host can route to, so a published
    /// port is the only way traffic gets in — where a rootful one has a
    /// bridge address and can be reached on any port at all. A protocol
    /// that let a port be named at request time would work in the
    /// second case and be quietly broken in the first, which is the
    /// worst way to be wrong: it compiles, and it works in whichever
    /// setup happens to be rootful.
    ///
    /// # More than one, because one is not the rule
    ///
    /// A container has at least two — its entrypoint's, and its
    /// proxy's, which carries everything else. A provider that arranges
    /// something further for itself needs somewhere to say so, and this
    /// is it.
    ///
    /// Order means nothing and duplicates mean nothing. Empty is a
    /// container nothing reaches over a socket, which is ordinary.
    ///
    /// # A port here does not mean anything is listening
    ///
    /// It means a provider undertook to make that port reachable if
    /// something binds it. Whether anything did is found out by
    /// connecting — the same distinction a
    /// [`ContainerDeployer`](super::container_deployer::ContainerDeployer)
    /// draws when it says a container running is not a server bound.
    pub ports: Vec<u16>,
}
