//! What a daemon connecting to another daemon through a provider is
//! made of, which neither end's wire owns.
//!
//! A daemon is a client of a provider, and so is another daemon; the
//! provider is the one thing both are connected to, so it is where one
//! reaches the other. The accepting daemon holds a `daemons::accept`
//! scope on the provider; the connecting one opens a `daemons::connect`
//! scope naming the acceptor by the identity the provider knows it by.
//! The provider tells the acceptor of the connector as a
//! [`Connection`]: an id the provider minted for the pair of channels
//! the connection rides, and the connector's identity and address as
//! the provider's own word.
//!
//! From then on the frames of a daemon connection cross the provider
//! as bytes it does not read, every type of the wire's own — the
//! connector's client frames one way, held as [`client::Owned`], the
//! acceptor's server frames the other, held as [`server::Owned`]. The
//! relayed connection is a connection like any on a socket, so it
//! begins with the [`handshake`](crate::wire::handshake): the
//! connector's first frame is its token, and the acceptor's first
//! frame is its own, from the same issuer. The provider relays both
//! unread; a connection either end does not accept is one that end
//! hangs up.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in; the two
//! directions' frames are their own modules, each an `Owned`.

mod connection;

pub mod client;
pub mod server;

pub use connection::*;
