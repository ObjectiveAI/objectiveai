//! Volume create: make one.
//!
//! A client names a volume, its size and its persist mode; the daemon
//! makes it, or has no room for it, or fails, and finishes. The
//! volume outlives the scope and every connection, until a delete.
//!
//! The exchange is the provider's [`diverge_provider_sdk::endpoints::volumes::create`] — the same request
//! after the tag, the same answers — and its rule in full is stated
//! there; only the tag and the host differ. Split by who SENDS, as
//! everywhere else: the ask is in [`client`], the answer in
//! [`server`].

pub mod client;
pub mod server;
