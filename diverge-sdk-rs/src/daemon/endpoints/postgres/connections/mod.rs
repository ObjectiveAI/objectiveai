//! Listing the container connections open through the database. One
//! request, a stream of answers. A client asks; the daemon sends one
//! [`Connection`](crate::daemon::endpoints::postgres::Connection) per
//! container connection open through the database it serves now, oldest
//! opened first, and finishes — or finishes with nothing when none is
//! open. What a set would be refused over is exactly this list at that
//! moment; it is what a client reads to know what to end, and reads
//! again to know it is gone. There is no filter and no count: the list
//! is whole, and short.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
