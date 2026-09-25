//! Volume read: read a file out of one.
//!
//! A client names a volume of the daemon's and a file in it; the
//! daemon streams the file's bytes back, the volume held to itself,
//! and finishes.
//!
//! The exchange is the provider's [`diverge_provider_sdk::endpoints::volumes::read`] — the same request
//! after the tag, the same answers — and its rule in full is stated
//! there; only the tag and the host differ. Split by who SENDS, as
//! everywhere else: the ask is in [`client`], the answer in
//! [`server`].

pub mod client;
pub mod server;
