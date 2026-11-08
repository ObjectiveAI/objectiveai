//! Listing the images in the caller's Store, and keeping the listing.
//!
//! One question with no parameters, and a stream kept open: every
//! image the caller owns, the word that the listing is whole, then
//! every image built, pushed, edited or deleted after, until the
//! caller cancels.
//! Split by who SENDS, as everywhere else: the question and the cancel
//! are in [`client`], the answer in [`server`].

pub mod client;
pub mod server;
