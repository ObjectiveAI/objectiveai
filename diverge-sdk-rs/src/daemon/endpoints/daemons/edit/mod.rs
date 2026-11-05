//! Changing a remote daemon's record: its description, the remote it
//! names, or the account of ours it is reached as. Its name never
//! changes. A change drops a connection held to it, so the next use
//! connects anew.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
