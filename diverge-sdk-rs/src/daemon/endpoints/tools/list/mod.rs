//! Listing a caller's tools.
//!
//! A client asks for its tools, narrowed, and the daemon sends every
//! one it holds under the caller's identity that the request's
//! filter lets through — by name, by template, by creator, by origin, by
//! activity, by the agents it is attached to, by tags, all or any, by when it
//! was created — one response each, oldest created first, and
//! finishes: what each is called, where it comes from — the template
//! it was made from, or the container of somebody else's it joins —
//! whether it is active now, when that last changed, which agents
//! it is attached to, and its tags. A jq program on the request
//! runs over each tool the filter lets through, and what it yields
//! is what comes back; a count caps what comes back. A request that
//! says nothing is every tool. A caller with no tool that matches
//! sees the finish and nothing before it. The daemon does not stay
//! open.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
