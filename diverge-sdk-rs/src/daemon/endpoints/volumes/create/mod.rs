//! Creating a volume on a provider. One request, one answer. A client
//! names a provider, a name, a size and a mode; the daemon asks the
//! provider for the volume, and answers that it exists, that no
//! provider is the one named, that the name is a volume's on that
//! provider already, that the provider cannot reserve that many bytes,
//! forbidden, or that it failed, and the scope finishes. The volume is
//! the provider's from then on, and outlives every container and
//! connection; only a [`delete`](super::delete) ends it.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
