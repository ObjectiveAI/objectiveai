//! What a daemon connecting to another daemon through a provider is
//! made of, which neither end's wire owns.
//!
//! A daemon is a client of a provider, and so is another daemon; the
//! provider is the one thing both are connected to, so it is where one
//! reaches the other. The accepting daemon holds a `daemons::accept`
//! scope on the provider; the connecting one opens a `daemons::connect`
//! scope naming the acceptor by the identity the provider knows it
//! by, and [`Mode`], how the connector authenticates to the acceptor —
//! the credential of an account the connector holds on it. The
//! provider tells the acceptor of the connector as a [`Connection`]:
//! an id the provider minted for the pair of channels the connection
//! rides, the connector's identity and address as the provider's own
//! word, and the mode as the connector asserted it. From then on the
//! frames of a daemon connection cross the provider as bytes it does
//! not read: the connector's client frames one way, the acceptor's
//! server frames the other, the forms
//! [`containers::daemon`](crate::shared::containers::daemon) defines
//! for a container program's daemon connection — a daemon connection
//! minus its `Auth` frame, which is what the mode stands in for.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod connection;
mod mode;

pub use connection::*;
pub use mode::*;
