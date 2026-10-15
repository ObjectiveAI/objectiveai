//! Building an image on the provider, from a Containerfile.
//!
//! A caller names an image, hands the Containerfile, and names the
//! directory of one of its volumes as the build's context if the
//! Containerfile copies from one; the provider builds, saying what
//! the build says as it goes, and answers the image built — its
//! digest and its size — or that the store has no room, or an
//! error. Split by who SENDS, as everywhere else: the ask is in
//! [`client`], the answers in [`server`].

pub mod client;
pub mod server;
