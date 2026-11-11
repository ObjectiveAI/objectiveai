//! The server side of a daemons list: what a provider sends.
//!
//! [`response`] is the whole of it: an identity added or removed, or
//! the word that the listing is whole. A provider opens no channel of
//! its own on a listing; the one channel on it is the caller's cancel.
//! The provider crate's `handle` performs the exchange, from the
//! identities that hold a
//! [`daemons::announce`](crate::provider::endpoints::daemons::announce)
//! open on it, and keeps answering until the caller cancels.

pub mod response;
