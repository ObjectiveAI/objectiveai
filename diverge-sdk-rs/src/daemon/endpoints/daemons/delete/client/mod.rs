//! The client side of a daemons delete: what a client sends.
//!
//! [`request`] is the whole of what goes on the wire; [`execute`]
//! performs the exchange and hands back the daemon's answer.

pub mod request;

pub mod execute;
