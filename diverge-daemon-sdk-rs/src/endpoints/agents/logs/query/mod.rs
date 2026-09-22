//! Reading an agent's log through a jq program.
//!
//! A client names an agent of its own and hands the daemon a jq
//! program; the daemon runs the program over every item of the log,
//! oldest first, and sends every value the program yields, one
//! response each, in order, then finishes. The program is the whole
//! of the query — a filter, a projection, a reduction, whatever jq
//! says — and what comes back is what it yields, not the items it
//! read. Nothing is held open: a query answers what the log holds at
//! the time of the request. An agent that does not exist, or a
//! program that will not compile or fails while it runs, is the
//! scope's error.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
