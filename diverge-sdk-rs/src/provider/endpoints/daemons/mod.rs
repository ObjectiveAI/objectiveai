//! Daemons reaching one another through the provider.
//!
//! Two scopes, one per end. A daemon that will take connections from
//! other daemons holds an [`accept`] on the provider; a daemon that
//! wants one opens a [`connect`] naming the acceptor by the identity
//! the provider knows it by, and the provider relays the daemon
//! connection between the two for as long as both scopes live —
//! bytes it does not read, in the forms
//! [`shared::daemons`](crate::shared::daemons) states. Nothing of a
//! daemon's own wire is here: the connection carried is the daemon
//! protocol, whole, and the provider is the one thing both ends
//! happen to be connected to.

pub mod accept;
pub mod connect;
