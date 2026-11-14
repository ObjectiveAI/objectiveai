//! Getting one image in a Store of the daemon's. One request, one
//! answer. A client names a provider and a digest; the daemon answers
//! with the image as a list would report it, that there is none,
//! forbidden, or that it failed, and the scope finishes.
//!
//! The rules:
//!
//! 1. **Read from what the daemon holds**, as a
//!    [`list`](super::list) is: the listing of its Store the daemon
//!    keeps for each connected provider. The provider is not asked.
//! 2. **Not found** is a provider the daemon keeps no record of, or a
//!    Store whose listing holds no image of the digest.
//! 3. **A failure** is a provider that is not connected now, that has
//!    no Store, or whose listing is not whole yet: the daemon could not
//!    look.
//! 4. **What it is not.** It answers the list's row and nothing of the
//!    image itself: [`read`](super::read) does that.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
