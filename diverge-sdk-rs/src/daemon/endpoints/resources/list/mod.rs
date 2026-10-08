//! Listing resources. A client asks for the resources, narrowed, and
//! the daemon sends every one its `list` grants reach that the
//! request's filter lets through — by id, by kind, by whether a
//! container mounts it, by creator, by tags, by when it was first held
//! — one response each, oldest first, then the word that the list is
//! whole, and keeps the scope open: each resource added, changed or
//! removed, as resources are held, described, tagged and deleted and as
//! the containers that mount them come and go, until the client
//! cancels, the one channel it opens on the scope. Each is the resource
//! by its id, its kind, its description, when it was first held and by
//! whom, its tags, and how many bytes it holds. A count keeps the list
//! to the first that many that match. A request that says nothing is
//! every resource the grants reach. A caller whose grants reach none
//! that matches is told the list is whole at once, and watched.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
