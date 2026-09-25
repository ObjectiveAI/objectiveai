//! Volume list: list the daemon's volumes.
//!
//! A client asks which volumes the daemon holds for it; the daemon
//! answers with every one, by name, size, age and persist mode, or
//! an error, and finishes.
//!
//! The exchange is the provider's [`diverge_provider_sdk::endpoints::volumes::list`] — the same request
//! after the tag, the same answers — and its rule in full is stated
//! there; only the tag and the host differ. Split by who SENDS, as
//! everywhere else: the ask is in [`client`], the answer in
//! [`server`].

pub mod client;
pub mod server;
