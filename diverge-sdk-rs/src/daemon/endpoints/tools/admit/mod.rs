//! Admitting a lister or a connector to a tool the daemon runs. One
//! request, one answer. A client names a tool of its own and gives an
//! [`Admission`](crate::daemon::endpoints::tools::Admission) — an
//! identity on the tool's provider, an address if one, and what it
//! admits — and the daemon answers that the admission is made, with the
//! key it minted when the admission admits a connect, answered here and
//! never again; that no tool of the caller's is the one named; that the
//! tool is a connected one, somebody else's, whose runner admits and
//! the daemon cannot; that an admission for that identity is on the
//! tool already; forbidden; or that it failed, and the scope finishes.
//!
//! # What an admission answers
//!
//! A provider the daemon runs a tool on asks the daemon, over the
//! tool's run scope, whether a lister may see the container and whether
//! a connector may attach to it — the provider protocol's
//! `authorize_list` and `authorize_connect`. The daemon answers yes to
//! a list when an admission on the tool names the lister's identity and
//! admits a list, and yes to a connect when the connector's
//! `authorization` is an admission's key and the admission admits a
//! connect; an address on the admission bounds either to the one the
//! provider saw. Everything else is no. An admission is taken back by
//! [`unadmit`](crate::daemon::endpoints::tools::unadmit).
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
