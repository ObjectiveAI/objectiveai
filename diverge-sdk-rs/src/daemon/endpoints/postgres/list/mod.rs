//! Listing the container connections open through the database, and
//! keeping the list. One request, a stream kept open. A client asks;
//! the daemon sends one
//! [`Connection`](crate::daemon::endpoints::postgres::Connection) per
//! container connection open through the database it serves now,
//! oldest opened first, then the word that the list is whole — at
//! once, when none is open — and from then on each connection added
//! as it opens and removed as it closes, until the client cancels, the
//! one channel it opens on the scope. It is what a client reads to
//! know what is on the database now, and keeps reading. There is no
//! filter and no count: the list is whole, and short.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
