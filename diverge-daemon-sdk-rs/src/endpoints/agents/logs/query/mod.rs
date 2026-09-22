//! Reading an agent's log from an id on.
//!
//! A client names an agent of its own and, optionally, the id of the
//! last item it has; the daemon sends every item of the log after
//! that id, oldest first, one response each, and finishes. Nothing
//! is held open: a query answers what the log holds at the time of
//! the request, and a client that wants what comes after asks again
//! from the last id it received. An agent that does not exist is the
//! scope's error.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
