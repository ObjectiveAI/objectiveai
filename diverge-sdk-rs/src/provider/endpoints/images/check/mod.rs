//! Checking whether a provider can supply an image.
//!
//! One question, one answer, and the question says where to look:
//! the client's own images, the registries, or both, and never what
//! the provider holds on its own account. Availability and the terms attached to it
//! are not separate round trips: a provider that cannot supply an
//! image has nothing to quote, so a caller asking only about terms
//! would still need the availability answer to read them.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and a provider answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
