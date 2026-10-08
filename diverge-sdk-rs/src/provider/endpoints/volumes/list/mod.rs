//! Listing the directories a provider offers, and keeping the
//! listing.
//!
//! One question with no parameters, and a stream kept open: every
//! volume the caller has, the word that the listing is whole, then
//! every volume created, edited or deleted after, until the caller
//! stops. Split by who SENDS, as everywhere else: the question and
//! the stop are in [`client`], the answer in [`server`].

pub mod client;
pub mod server;
