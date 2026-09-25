//! Volume write: write a file into one.
//!
//! A client names a volume of the daemon's and a destination in it;
//! the daemon asks for the content on a channel of its own, puts the
//! file in place whole, and says so, the volume held to itself
//! throughout.
//!
//! The exchange is the provider's [`diverge_provider_sdk::endpoints::volumes::write`] — the same request
//! after the tag, the same answers — and its rule in full is stated
//! there; only the tag and the host differ. Split by who SENDS, as
//! everywhere else: the ask is in [`client`], the answer in
//! [`server`].

pub mod client;
pub mod server;
