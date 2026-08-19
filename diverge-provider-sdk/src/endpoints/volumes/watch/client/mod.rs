//! The client side of a watch: what a client sends.
//!
//! [`request`] opens it — one name, once. [`channel_request`] closes
//! it, and is the only other thing a caller ever sends: everything in
//! between travels the other way.
//!
//! It is the only volume endpoint with a second half at all, because it
//! is the only one that does not end by itself. The rest answer and
//! finish; a watch reports until somebody says stop.
//!
//! # And, behind the `client` feature, a way to use it
//!
//! [`execute`] starts the watch and hands back an [`ExecuteStream`]
//! of what the provider says about the tree. It is the one volume
//! endpoint that does not collapse into a single answer, because a
//! watch does not end.
//!
//! It sends the [`channel_request`] when that stream is dropped, which
//! makes dropping the ordinary way to be done with a watch rather than
//! a way to abandon one.
//!
//! Every other module in [`endpoints`](crate::endpoints) is types only,
//! and this one still is unless a caller asked for the half of the
//! crate that can hold a socket — the same bargain
//! [`client`](crate::client) itself makes. It is flattened in rather
//! than kept behind a name of its own, because a name for it would be
//! a name for one function.

pub mod channel_request;
pub mod request;

#[cfg(feature = "client")]
mod execute;
#[cfg(feature = "client")]
mod execute_stream;

#[cfg(feature = "client")]
pub use execute::*;
#[cfg(feature = "client")]
pub use execute_stream::*;
