//! The client side of a providers enable: what a client sends.
//!
//! [`request`] is the whole of what goes on the wire; [`execute`]
//! performs the exchange and hands back the daemon's answer.

pub mod request;

pub mod execute;
