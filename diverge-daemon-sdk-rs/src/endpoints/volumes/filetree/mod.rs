//! Volume filetree: see what one holds.
//!
//! A client names a volume of the daemon's and a subtree of it; the
//! daemon answers with the tree as it is, once, the volume held to
//! itself, and finishes. Nothing is watched and no change is sent.
//!
//! The exchange is the provider's [`diverge_provider_sdk::endpoints::volumes::filetree`] — the same request
//! after the tag, the same answers — and its rule in full is stated
//! there; only the tag and the host differ. Split by who SENDS, as
//! everywhere else: the ask is in [`client`], the answer in
//! [`server`].

pub mod client;
pub mod server;
