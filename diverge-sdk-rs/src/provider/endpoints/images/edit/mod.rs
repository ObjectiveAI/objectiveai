//! Changing what the caller says of an image in its Store.
//!
//! A caller names an image by its digest and gives a new description,
//! or none to clear it; the provider keeps it, and the listing says
//! [`Changed`](super::list::server::response::Frame::Changed). The
//! description is the one thing about an image that changes: its
//! bytes are its digest's. Split by who SENDS, as everywhere else:
//! the ask is in [`client`], the answer in [`server`].

pub mod client;
pub mod server;
