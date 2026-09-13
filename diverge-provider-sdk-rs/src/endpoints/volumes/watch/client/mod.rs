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
//! [`execute`] starts the watch and hands back an [`ExecuteStream`](execute::ExecuteStream)
//! of what the provider says about the tree. It is the one volume
//! endpoint that does not collapse into a single answer, because a
//! watch does not end.
//!
//! It hands back an [`ExecuteHandle`](execute::ExecuteHandle) beside
//! it, whose one method sends the [`channel_request`] that stops the
//! watch. Ending one is something a caller says rather than something
//! that happens when a value goes out of scope — a destructor could
//! not await it, could not report it failing, and could not be told to
//! leave the watch running.
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
pub mod execute;
