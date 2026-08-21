//! What every container asks for, whatever is going to run in it.

use indexmap::IndexMap;

use crate::shared::container::request::Mount;

/// A container to put somewhere, minus the image.
///
/// What the three container endpoints mean by "a container" once the
/// differences between them are set aside. An
/// [`agentic_loop`](crate::endpoints::agentic_loop::run) runs an agent
/// in one, a [`laboratory`](crate::endpoints::laboratories::run) is one
/// an agent works inside, and an
/// [`mcp_plugin`](crate::endpoints::mcp_plugin::run) is one that serves
/// tools — and none of that is here, because none of it changes how the
/// container is deployed.
///
/// # It is built, not passed through
///
/// No endpoint's request frame becomes one of these. A handler reads
/// its own request and fills this in, and for
/// [`agentic_loop`](crate::endpoints::agentic_loop::run) there is
/// nothing to copy at all — its request carries an agent, a prompt and
/// a continuation, and what container that implies is the provider's
/// to work out.
///
/// Which is the point. A deployer that took a request frame would take
/// three of them, and be three deployers.
///
/// # The image is not here
///
/// It is an argument to whichever
/// [`ContainerDeployer`](super::container_deployer::ContainerDeployer)
/// method is called, because the method IS the source. Carrying an
/// [`Image`](crate::shared::container::request::Image) here as well
/// would let a caller hand
/// [`Client`](crate::shared::container::request::Image::Client) to the
/// method that pulls from a registry, which is a contradiction nothing
/// would catch.
///
/// # What is deliberately absent
///
/// A plugin's `port`, `arguments` and `identity` are not container
/// facts. The endpoint already says a provider delivers the last two
/// through the environment "by its own reserved names", so a handler
/// folds them in and what arrives here is
/// [`environment`](Self::environment).
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
    /// Empty for a container that takes none, which is every plugin —
    /// a plugin serves tools rather than works on a filesystem, and
    /// mounting a caller's directories into one would hand it access it
    /// has no reason to want.
    ///
    /// See [`Mount`] for why the host side is a name and an offset
    /// rather than a path.
    pub mounts: Vec<Mount>,
}
