//! Examining one image in the caller's Store.
//!
//! [`list`](super::list) says which images exist; this says what one
//! was built from. A caller names an image by its digest and gets the
//! listing's four fields back with one more on top — the
//! Containerfile, for an image built on the provider. Split
//! by who SENDS, as everywhere else: the ask is in [`client`], the
//! answer in [`server`].

pub mod client;
pub mod server;
