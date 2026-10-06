//! Getting one resource. One request, one answer. A client names a
//! resource by its id; the daemon answers with the resource as a list
//! would report it, that no resource has the id, forbidden, or that it
//! failed, and the scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
