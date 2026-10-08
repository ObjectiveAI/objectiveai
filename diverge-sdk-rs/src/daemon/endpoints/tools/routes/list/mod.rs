//! Listing a caller's routes.
//!
//! A client asks for its routes, narrowed, and the daemon sends every
//! one it holds under the caller's identity that the request's filter
//! lets through — by the agent a path begins at, by the template it
//! ends at, by the tool it routes to, by creator, by when it was put
//! down — one response each, oldest first, then the word that the list
//! is whole, and keeps the scope open: each route added, changed or
//! removed, as routes are set and deleted and the tools they route to
//! are renamed and deleted, until the client cancels, the one channel
//! it opens on the scope. Each is its path, the tool it routes to as
//! it is called now, when it was put down and by whom. A count keeps
//! the list to the first that many that match. A request that says
//! nothing is every route. A caller with no route that matches is told
//! the list is whole at once, and watched.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
