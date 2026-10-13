//! Listing a caller's dependency templates.
//!
//! A client asks for its dependency templates, narrowed, and the
//! daemon sends every one it holds under the caller's identity that
//! the request's filter lets through — by id, by creator, by type, by
//! scope, by whether any agent depends on it, by the agents that do,
//! by whether any tool runs from it, by tags, all or any, by when it
//! was made — one response each, oldest made first, then the word that
//! the list is whole, and keeps the scope open: each template added,
//! changed or removed, as registrations make templates and reset what
//! agents depend on, as templates are tagged and deleted, and as the
//! tools running from them start and stop, until the client cancels,
//! the one channel it opens on the scope. Each is the template by its
//! id, when it was made, who made it, its tags, the template whole,
//! the agents whose last registration declared it, and the tools
//! running from it now, each by whose it is and where it runs. A count
//! keeps the list to the first that many that match. A request that
//! says nothing is every template. A caller with no template that
//! matches is told the list is whole at once, and watched.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
