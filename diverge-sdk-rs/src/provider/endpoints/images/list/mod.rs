//! Listing the images a caller built, and keeping the listing.
//!
//! One question with no parameters, and a stream kept open: every
//! image the caller has built, the word that the listing is whole,
//! then every image built or deleted after, until the caller stops.
//! Split by who SENDS, as everywhere else: the question and the stop
//! are in [`client`], the answer in [`server`].

pub mod client;
pub mod server;
