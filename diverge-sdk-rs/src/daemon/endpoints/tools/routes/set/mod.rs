//! Setting a route: at one position, this tool. One request, one
//! answer. A client names a position — a [`Path`](super::Path) — and a
//! tool of its own; the daemon answers that the route is down, that no
//! tool of the caller's is the one named, that the tool is not of the
//! position's template, that the position has a route already, or that
//! it failed, and the scope finishes. From then on a run reaching that
//! position is served the tool and asks no deployer.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
