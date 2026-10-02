//! Listing a caller's templates.
//!
//! A client asks for its templates, narrowed, and the daemon sends
//! every one it holds under the caller's identity that the request's
//! filter lets through — by id, by whether any tool was made from
//! it, by tags, by when it was made — one response each, oldest
//! made first, and finishes: each by its id, when it was made, its
//! tags, and the template whole. A jq program on the request runs
//! over each template the filter lets through, and what it yields
//! is what comes back; a count caps what comes back. A request that
//! says nothing is every template. A caller with no template that
//! matches sees the finish and nothing before it. The daemon does
//! not stay open.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
