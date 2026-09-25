//! Volume stat: examine one.
//!
//! A client names a volume of the daemon's; the daemon answers with
//! the listing's fields and, on top, the bytes in use and the hash of
//! the content, walked with the volume held to itself, and finishes.
//!
//! The exchange is the provider's [`diverge_provider_sdk::endpoints::volumes::stat`] — the same request
//! after the tag, the same answers — and its rule in full is stated
//! there; only the tag and the host differ. Split by who SENDS, as
//! everywhere else: the ask is in [`client`], the answer in
//! [`server`].

pub mod client;
pub mod server;
