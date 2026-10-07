//! Taking an admission back. One request, one answer. A client names a
//! tool of its own and an identity; the daemon answers that no
//! admission for that identity is on the tool from then on — whether or
//! not one was, since an admission that was not there is nothing to
//! take back — that no tool of the caller's is the one named,
//! forbidden, or that it failed, and the scope finishes. A connector
//! attached through the admission's key now is not dropped; the next
//! one is refused.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
