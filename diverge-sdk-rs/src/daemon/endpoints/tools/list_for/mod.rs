//! Asking a provider which tool containers somebody runs.
//!
//! A client names a provider of its own and an identity on that
//! provider; the daemon opens the provider protocol's
//! [`containers::tools::list_for`](crate::provider::endpoints::containers::tools::list_for)
//! on that provider with the identity, and relays what comes back as
//! it comes: one response per tool container the provider adds, in
//! the order it does, the provider's word that the listing is whole,
//! and then every container added and removed as the identity's runs
//! begin and end, for as long as the client keeps the scope — a
//! listing is a stream kept open, and ends at the client's cancel,
//! the one channel a client opens on it, which the daemon passes on
//! to the provider as its stop. A provider
//! the caller does not have is answered by exactly
//! one response saying so, and the finish. What is listed is a
//! container somebody else runs, named by the id its runner was given:
//! what a [`connect`](super::connect) then offers, with an
//! authorization, and the runner is asked.
//!
//! Nothing is narrowed here and nothing is transformed: the daemon is a
//! relay, and which containers are told of is each runner's to say,
//! container by container, on the provider.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
