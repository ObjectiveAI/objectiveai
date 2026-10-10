//! Listing a caller's tools.
//!
//! A client asks for its tools, narrowed, and the daemon sends every
//! one it holds under the caller's identity that the request's filter
//! lets through — by name, by template, by creator, by origin, by
//! activity, by the agents it is attached to, by tags, all or any, by
//! when it was created — one response each, oldest created first,
//! then the word that the list is whole, and keeps the scope open:
//! each tool added, changed or removed, as the records, the
//! attachments, the admissions and the runs change — a dependency
//! tool among them only while its agent's container runs — until
//! the client cancels, the one channel it opens on the scope. Each is
//! what the tool is called, where it comes from — the template it was
//! made from, or the container of somebody else's it joins — whether
//! it is active now, when that last changed, which agents it is
//! attached to, its admissions, and its tags. A count
//! keeps the list to the first that many that match. A request that
//! says nothing is every tool. A caller with no tool that matches is
//! told the list is whole at once, and watched.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
