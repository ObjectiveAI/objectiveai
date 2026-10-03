//! Listing a caller's routes.
//!
//! A client asks for its routes, narrowed, and the daemon sends every
//! one it holds under the caller's identity that the request's filter
//! lets through — by the agent a path begins at, by the template it
//! ends at, by the tool it routes to, by creator, by when it was put
//! down — one response each, oldest first, and finishes: each its path,
//! the tool it routes to, when it was put down and by whom. A jq
//! program on the request runs over each route the filter lets through,
//! and what it yields is what comes back; a count caps what comes back.
//! A request that says nothing is every route. A caller with no route
//! that matches sees the finish and nothing before it. The daemon does
//! not stay open.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
