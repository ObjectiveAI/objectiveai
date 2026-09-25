//! Volume delete: destroy one.
//!
//! A client names a volume of the daemon's; the daemon destroys it,
//! or answers that it is mounted, or fails, and finishes.
//!
//! The exchange is the provider's [`diverge_provider_sdk::endpoints::volumes::delete`] — the same request
//! after the tag, the same answers — and its rule in full is stated
//! there; only the tag and the host differ. Split by who SENDS, as
//! everywhere else: the ask is in [`client`], the answer in
//! [`server`].

pub mod client;
pub mod server;
