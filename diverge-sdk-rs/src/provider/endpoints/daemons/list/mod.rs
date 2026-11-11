//! Listing a provider's announced connected identities, and keeping
//! the listing.
//!
//! One question with no parameters, and a stream kept open, shaped as
//! [`images::list`](super::images::list): every identity connected to
//! the provider that holds a [`daemons::announce`](super::announce)
//! open, with the announcement that scope carries, the word that the
//! listing is whole, then every identity that comes to announce or
//! stops after, until the caller cancels.
//!
//! # Listed is announced
//!
//! An identity is listed for exactly as long as it holds a
//! [`daemons::announce`](super::announce) open — whichever end dialled
//! the connection — with the announcement that scope carries. An
//! identity connected without one is never listed, and an identity
//! that is not connected is never listed.
//!
//! Split by who SENDS, as everywhere else: the question and the cancel
//! are in [`client`], the answer in [`server`].

pub mod client;
pub mod server;
