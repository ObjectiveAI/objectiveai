//! The server side of a discover listing: what a provider sends.
//!
//! [`response`] is the whole of it: an identity added or removed, or
//! the word that the listing is whole. A provider opens no channel of
//! its own on a listing; the one channel on it is the caller's cancel.
//! The provider crate's `handle` performs the exchange, from the
//! identities it holds connected with the discover flag `1`, and
//! keeps answering until the caller cancels.

pub mod response;
