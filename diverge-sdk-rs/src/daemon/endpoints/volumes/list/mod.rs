//! Listing volumes, and keeping the list. A client asks for the
//! volumes, narrowed, and the daemon sends every volume of every
//! provider the caller's `list` grants reach that the request's filter
//! lets through — by provider, by name, by mode, by whether a
//! container of the daemon's mounts it, by when it was created — one
//! response each, provider by provider in the order the daemon knows
//! them and within a provider by name, then the word that the list is
//! whole, and keeps the scope open: each volume added, changed or
//! removed, as the providers' listings, the connections and the
//! mounting records change, until the client cancels, the one channel
//! it opens on the scope. Each is the volume as the provider lists it,
//! with the agents and the tools that mount it. The daemon holds one
//! listing of each connected provider's volumes for the connection's
//! life and reads from that, so a provider on record that is not
//! connected now, or has not yet said its listing is whole, contributes
//! nothing until it does. A count keeps the list to the first that
//! many that match. A request that says nothing is every volume the
//! grants reach. A caller whose grants reach none that matches is told
//! the list is whole at once, and watched.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
