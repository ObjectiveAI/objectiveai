//! A container that is running, and what can be done with one.

use std::future::Future;

/// A running container, as far as this crate needs one.
///
/// What a
/// [`ContainerDeployer`](super::container_deployer::ContainerDeployer)
/// hands back. The type itself is the provider's — a process handle, a
/// name, an instance id, a struct with all three — and this is the part
/// of it this crate has to be able to reach: where its proxy answers,
/// and how to end it.
///
/// # Everything else goes through the proxy
///
/// A container carries the proxy — the deployer put it there and
/// started it before the deploy returned — and every exchange with
/// the container is an exchange with the proxy over the one
/// WebSocket dialled to it: the begin scope the proxy's asks and the
/// agent's conversation ride, a scope per FUSE mount, the tree, a
/// read, a write. All of that is spoken by the executors under
/// [`container_proxy_endpoints`](crate::container_proxy::outside),
/// on the [`Handle`](crate::wire::client::handle::Handle) that
/// [`proxy::dial`](super::proxy::dial) makes from
/// [`address`](Self::address). So this trait names no exchange:
/// there was a version that did — MCP asks and answers, the agentic
/// loop, a postgres pair, commands, the filetree, reads and writes,
/// each a method with its own streams — and every one of them was the
/// proxy's protocol restated as a trait, to be implemented by
/// dialling the proxy. The dial is the whole of it, so the address is
/// the whole of it.
///
/// # Why an address is a method and never a field
///
/// Because how a provider reaches a container's port is the
/// provider's: published to a loopback port it picked, routed to a
/// bridge address, something a cloud runtime does that resembles
/// neither. The deploy is the only thing that learned which, and this
/// is where that fact is kept.
///
/// # `Send` and `Sync`
///
/// Because a scope is served by more than one task — one reading the
/// caller's channels, one relaying the proxy's asks, one pumping an
/// answer — and a container is reached from any of them, behind a
/// shared reference.
///
/// Which is also why every method takes `&self`. Nothing here consumes
/// a container, including [`stop`](Self::stop): it is stopped while
/// whatever is holding it still holds it, and dropping it afterwards is
/// a separate act that this crate does not define.
pub trait Container: Send + Sync {
    /// Where the proxy inside it answers, as this provider reaches it.
    ///
    /// The URL [`proxy::dial`](super::proxy::dial) takes —
    /// `ws://host:port`, dialled as given at its root path — naming
    /// the container's
    /// [`OUTSIDE_PORT`](crate::container_proxy::outside::OUTSIDE_PORT)
    /// however the runtime made it reachable. A fact the deploy
    /// learned, so it cannot fail and need not wait.
    fn address(&self) -> &str;

    /// End it.
    ///
    /// The container is gone when this resolves: as far as the
    /// provider's runtime is concerned, that container is done.
    /// Infallible, because there is nothing a handler could do with
    /// the failure — the scope is ending either way, and a container a
    /// runtime would not stop is the provider's to notice.
    ///
    /// # What it does to the scope is not this
    ///
    /// A scope ends when its provider finishes it, and stopping a
    /// container is one of the things that leads to that. This does not
    /// send a frame and does not know there is one to send.
    fn stop(&self) -> impl Future<Output = ()> + Send;
}
