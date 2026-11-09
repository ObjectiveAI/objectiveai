//! Listing peers through the daemon's providers. A client asks,
//! and the daemon sends every identity listable on any provider it
//! is connected to — one response each, every provider it was seen on
//! with it — then the word that the listing is whole, and keeps the
//! scope open until the client cancels, the one channel it opens on
//! the scope.
//!
//! The rules:
//!
//! 1. **Every connected provider.** The listing is the union of every
//!    connected provider's own `daemons::list`: the identities
//!    connected to it whose authorization frame carried the listable
//!    flag `1`.
//! 2. **One item per identity.** An identity seen on several providers
//!    is one item, its providers all listed with it, and is
//!    [`Changed`](server::response::Frame::Changed) as that list grows
//!    or shrinks.
//! 3. **A provider gone is its identities gone.** When a provider
//!    disconnects, an identity seen only through it is removed, and one
//!    seen elsewhere too is changed.
//! 4. **Never this daemon.** The daemon's own identity at a provider
//!    is not listed.
//! 5. **What it is for.** An item is what a `daemons.connect` remote
//!    of the daemon's configuration is made from: its `issuer`, and one
//!    link per provider — that provider, and the item's identity as the
//!    remote's there — by which the remote is relayed to, and its tools
//!    connected.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
