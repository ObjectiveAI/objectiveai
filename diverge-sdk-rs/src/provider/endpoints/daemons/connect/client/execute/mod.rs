//! Performing the exchange, rather than describing it.
//!
//! [`execute`] connects to another daemon and hands back a
//! [`Connected`]: a [`Handle`](crate::wire::client::handle::Handle) on
//! that daemon — a client of the daemon protocol, its frames carried
//! inside the scope — and the connection's end.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod connected;
mod execute;

pub use connected::*;
pub use execute::*;
