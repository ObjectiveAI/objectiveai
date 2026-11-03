//! The server side of a discover request: what a provider sends.
//!
//! [`response`] is the whole of it. A provider answers and is done; it
//! opens no channels. The provider crate's `handle` performs the
//! exchange, from the identities it holds connected with the discover
//! flag `1`.

pub mod response;
