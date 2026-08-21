//! A container that is running, and what can be done with one.

use std::future::Future;

/// A running container, as far as this crate needs one.
///
/// What a
/// [`ContainerDeployer`](super::container_deployer::ContainerDeployer)
/// hands back. The type itself is the provider's — a process handle, a
/// name, an instance id, a struct with all three — and this is the part
/// of it this crate has to be able to reach.
///
/// # One method, and it is the one nothing else can do
///
/// A container has to be stopped, and only whoever deployed it knows
/// how. Everything else a container is asked for goes through channels
/// the scope already carries: an MCP exchange, a file read, a file
/// written, a filetree. Those are relayed, and relaying does not need a
/// method here.
///
/// So this will grow, but not by much, and not from the endpoints.
///
/// # `Send` and `Sync`
///
/// Because a scope is served by more than one task — a dispatcher
/// reading channel requests, a pump answering one — and a container is
/// reached from any of them, behind a shared reference.
///
/// Which is also why [`stop`](Self::stop) takes `&self`. Nothing here
/// consumes a container: it is stopped while whatever is holding it
/// still holds it, and dropping it afterwards is a separate act that
/// this crate does not define.
pub trait Container: Send + Sync {
    /// Why a container could not be stopped.
    ///
    /// The provider's own, for the reason
    /// [`ContainerDeployer::Error`](super::container_deployer::ContainerDeployer::Error)
    /// is: what goes wrong stopping a container belongs to a runtime,
    /// and this crate does not name one.
    ///
    /// It need not be the same type a deploy fails with. A provider
    /// whose deploy and stop go wrong in the same ways uses one type
    /// for both, and one whose stop can only fail in a way a deploy
    /// never could says so.
    ///
    /// [`Send`] and `'static` for the same reasons every other error
    /// here is: the future carrying it is [`Send`], so its output has
    /// to be, and it outlives the call that produced it.
    type Error: Send + 'static;

    /// Stop it.
    ///
    /// Returns when the container is stopped, the way a deploy returns
    /// when it is running. Not when a stop has been requested, not when
    /// a signal has been sent — an implementation that waits for
    /// something waits before it returns [`Ok`].
    ///
    /// # A container that is already gone is a stop that worked
    ///
    /// Because what a caller wanted is the state, not the act. A
    /// container that exited on its own, or crashed, or was stopped by
    /// something else, is a container that is not running — which is
    /// the whole of what was asked for.
    ///
    /// The alternative is an error every caller has to recognise and
    /// then ignore, which is a way of writing the same rule in every
    /// caller instead of once here.
    ///
    /// # What it does to the scope is not this
    ///
    /// A scope ends when its provider finishes it, and stopping a
    /// container is one of the things that leads to that. This does not
    /// send a frame and does not know there is one to send.
    ///
    /// # It takes `&self`
    ///
    /// A container is not consumed by being stopped. Whatever holds one
    /// goes on holding it — long enough to report the stop failing, at
    /// least — and what happens to it afterwards is that holder's.
    fn stop(&self) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
