//! Volume create capacity: ask how large one may be made.
//!
//! A client asks how large a volume the daemon could make for it now;
//! the daemon answers with a number of bytes, reserving nothing, and
//! finishes.
//!
//! The exchange is the provider's [`diverge_provider_sdk::endpoints::volumes::create_capacity`] — the same request
//! after the tag, the same answers — and its rule in full is stated
//! there; only the tag and the host differ. Split by who SENDS, as
//! everywhere else: the ask is in [`client`], the answer in
//! [`server`].

pub mod client;
pub mod server;
