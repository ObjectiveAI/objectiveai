//! Volume edit capacity: ask how far one may grow.
//!
//! A client names a volume of the daemon's; the daemon answers with
//! how many bytes it could grow by now, reserving nothing, and
//! finishes.
//!
//! The exchange is the provider's [`diverge_provider_sdk::endpoints::volumes::edit_capacity`] — the same request
//! after the tag, the same answers — and its rule in full is stated
//! there; only the tag and the host differ. Split by who SENDS, as
//! everywhere else: the ask is in [`client`], the answer in
//! [`server`].

pub mod client;
pub mod server;
