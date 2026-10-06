//! Listing volumes. A client asks for the volumes, narrowed, and the
//! daemon asks every provider the caller's `list` grants reach for its
//! listing and sends every volume the request's filter lets through —
//! by provider, by name, by mode, by whether a container of the
//! daemon's mounts it, by when it was created — one response each,
//! provider by provider in the order the daemon knows them and within a
//! provider in the order it listed, and finishes: each the volume as
//! the provider lists it, with the agents and the tools that mount it.
//! A count caps what comes back. A request that says nothing is every
//! volume the grants reach. A caller whose grants reach none that
//! matches sees the finish and nothing before it. A provider that could
//! not be asked is the scope's error, after whatever was sent. The
//! daemon does not stay open.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
