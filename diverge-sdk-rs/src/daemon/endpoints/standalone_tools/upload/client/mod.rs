//! The client side of an upload: what a client sends.
//!
//! [`request`] opens the scope: the tool, the destination, and a
//! directory's files. [`channel_response`] is what the client answers
//! each content channel the daemon opens with: the file's bytes in
//! pieces, then the finish.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand it
//! a [`Handle`](crate::wire::client::handle::Handle) and the request,
//! and get the daemon's answer.

pub mod channel_response;
pub mod request;

pub mod execute;
