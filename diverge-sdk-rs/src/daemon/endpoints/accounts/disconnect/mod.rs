//! Disconnecting an account. One request, one answer. A client names an
//! account — or an administrator of the configuration's with none — and
//! the daemon closes every connection a client has as it, answering how
//! many it closed, or that no account is the one named, forbidden, or
//! that it failed, and the scope finishes. The rules:
//!
//! 1. **Every connection** a client has as it is closed: one dialled to
//!    this daemon, and one another daemon made as it through a provider.
//! 2. **At once.** Each connection's session ends as when its socket
//!    drops: every scope it held is ended, and what they were doing stops
//!    as it would then.
//! 3. **Not waited for.** The daemon answers once it has told every
//!    connection to close, so an account may disconnect itself.
//! 4. **Nothing else changes.** The account stays as it is, and a client
//!    may connect as it again at once.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
