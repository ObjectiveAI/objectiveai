//! Swapping which database the daemon serves. One request, one answer.
//! A client gives a [`Mode`](crate::daemon::endpoints::postgres::Mode)
//! anew — local, or remote with a URL — and the daemon answers that it
//! serves it from now on, that it cannot swap because container
//! connections are open through the current one, or that it failed, and
//! the scope finishes. The mode is replaced whole; a remote URL is
//! given entire, password and all, which is how one rotates.
//!
//! # In use
//!
//! A set is refused as `InUse` while any container connection is open
//! through the current database, whichever mode the request gives —
//! even the current one — because a connection is a raw splice that
//! cannot be moved, and the answer names the containers holding one so
//! the client knows what to end. Nothing changed, and the client asks
//! again once they are gone. See
//! [`postgres`](crate::daemon::endpoints::postgres).
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
