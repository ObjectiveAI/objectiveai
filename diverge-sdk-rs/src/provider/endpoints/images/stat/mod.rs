//! Examining one image the caller built.
//!
//! [`list`](super::list) says which images exist; this says what one
//! was built from. A caller names an image and gets the listing's
//! four fields back with one more on top — the Containerfile. Split
//! by who SENDS, as everywhere else: the ask is in [`client`], the
//! answer in [`server`].

pub mod client;
pub mod server;
