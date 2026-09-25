//! Volume edit: resize one, or change its persist mode.
//!
//! A client names a volume of the daemon's and a change — its size,
//! its persist mode, or both; the daemon makes the change, the
//! volume held to itself, or refuses it whole, and finishes.
//!
//! The exchange is the provider's [`diverge_provider_sdk::endpoints::volumes::edit`] — the same request
//! after the tag, the same answers — and its rule in full is stated
//! there; only the tag and the host differ. Split by who SENDS, as
//! everywhere else: the ask is in [`client`], the answer in
//! [`server`].

pub mod client;
pub mod server;
